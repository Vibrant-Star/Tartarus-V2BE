//! Passive Raw Input diagnostics. Native events remain unchanged.
use std::{
    collections::HashMap,
    ffi::c_void,
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

#[repr(C)]
struct WindowClass {
    style: u32,
    proc: unsafe extern "system" fn(isize, u32, usize, isize) -> isize,
    class_extra: i32,
    window_extra: i32,
    instance: isize,
    icon: isize,
    cursor: isize,
    brush: isize,
    menu: *const u16,
    name: *const u16,
}
#[repr(C)]
#[derive(Default)]
struct Message {
    window: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[repr(C)]
struct Registration {
    page: u16,
    usage: u16,
    flags: u32,
    window: isize,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Device {
    handle: isize,
    kind: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const WindowClass) -> u16;
    fn UnregisterClassW(name: *const u16, instance: isize) -> i32;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: isize,
        menu: isize,
        instance: isize,
        param: *mut c_void,
    ) -> isize;
    fn DestroyWindow(window: isize) -> i32;
    fn DefWindowProcW(window: isize, message: u32, wparam: usize, lparam: isize) -> isize;
    fn GetMessageW(message: *mut Message, window: isize, min: u32, max: u32) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn RegisterRawInputDevices(devices: *const Registration, count: u32, size: u32) -> i32;
    fn GetRawInputDeviceList(devices: *mut Device, count: *mut u32, size: u32) -> u32;
    fn GetRawInputDeviceInfoW(
        device: isize,
        command: u32,
        data: *mut c_void,
        size: *mut u32,
    ) -> u32;
    fn GetRawInputData(
        input: isize,
        command: u32,
        data: *mut c_void,
        size: *mut u32,
        header_size: u32,
    ) -> u32;
    fn SetTimer(window: isize, id: usize, milliseconds: u32, callback: *const c_void) -> usize;
    fn KillTimer(window: isize, id: usize) -> i32;
    fn PostQuitMessage(code: i32);
    fn SetForegroundWindow(window: isize) -> i32;
    fn SetWindowTextW(window: isize, text: *const u16) -> i32;
    fn GetAsyncKeyState(key: i32) -> i16;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}

unsafe extern "system" fn window_proc(
    window: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == 2 {
        unsafe {
            PostQuitMessage(0);
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

struct Window {
    handle: isize,
    instance: isize,
    class: Vec<u16>,
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            KillTimer(self.handle, 1);
            DestroyWindow(self.handle);
            UnregisterClassW(self.class.as_ptr(), self.instance);
        }
    }
}
fn win_error(operation: &str) -> String {
    format!("{operation}: {}", std::io::Error::last_os_error())
}

fn target_path(path: &str) -> bool {
    let upper = path.to_ascii_uppercase();
    let parts: Vec<_> = upper.split(['#', '&', '\\']).collect();
    parts.contains(&"VID_1532") && parts.contains(&"PID_022B")
}

fn source_path(handle: isize) -> Result<String, String> {
    let mut size = 0;
    if unsafe { GetRawInputDeviceInfoW(handle, 0x20000007, std::ptr::null_mut(), &mut size) }
        == u32::MAX
    {
        return Err(win_error("GetRawInputDeviceInfoW size"));
    }
    if size == 0 || size > 32768 {
        return Err("Invalid source name length".into());
    }
    let mut buffer = vec![0u16; size as usize + 1];
    if unsafe { GetRawInputDeviceInfoW(handle, 0x20000007, buffer.as_mut_ptr().cast(), &mut size) }
        == u32::MAX
    {
        return Err(win_error("GetRawInputDeviceInfoW name"));
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).map_err(|e| e.to_string())
}

fn initial_devices() -> Result<Vec<Device>, String> {
    let mut count = 0;
    let size = std::mem::size_of::<Device>() as u32;
    if unsafe { GetRawInputDeviceList(std::ptr::null_mut(), &mut count, size) } == u32::MAX {
        return Err(win_error("GetRawInputDeviceList size"));
    }
    // Device arrival can race enumeration. Return the error rather than inventing an inventory.
    let mut devices = vec![Device::default(); count as usize];
    let result = unsafe { GetRawInputDeviceList(devices.as_mut_ptr(), &mut count, size) };
    if result == u32::MAX {
        return Err(win_error("GetRawInputDeviceList"));
    }
    devices.truncate(result as usize);
    Ok(devices)
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

pub fn capture(seconds: u32, output: &Path) -> Result<(), String> {
    record(seconds, output, false)
}

pub fn verify(seconds: u32, output: &Path) -> Result<(), String> {
    record(seconds, output, true)
}

fn record(seconds: u32, output: &Path, verify: bool) -> Result<(), String> {
    if !cfg!(target_arch = "x86_64") {
        return Err("Windows x64 required".into());
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let mut log = BufWriter::new(file);
    let io_err = |e: std::io::Error| e.to_string();
    writeln!(
        log,
        "# V2BE passive observer; duration={seconds}s verification={verify}"
    )
    .map_err(io_err)?;
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class: Vec<u16> = "TartarusV2BE.Diagnostics\0".encode_utf16().collect();
    let descriptor = WindowClass {
        style: 0,
        proc: window_proc,
        class_extra: 0,
        window_extra: 0,
        instance,
        icon: 0,
        cursor: 0,
        brush: 6,
        menu: std::ptr::null(),
        name: class.as_ptr(),
    };
    if unsafe { RegisterClassW(&descriptor) } == 0 {
        return Err(win_error("RegisterClassW"));
    }
    let handle = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            if verify { 0x10cf0000 } else { 0 },
            180,
            180,
            if verify { 900 } else { 0 },
            if verify { 260 } else { 0 },
            if verify { 0 } else { -3 },
            0,
            instance,
            std::ptr::null_mut(),
        )
    };
    if handle == 0 {
        unsafe {
            UnregisterClassW(class.as_ptr(), instance);
        }
        return Err(win_error("CreateWindowExW"));
    }
    let window = Window {
        handle,
        instance,
        class,
    };
    if verify {
        let static_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let text: Vec<u16> = "Tartarus V2BE - keyboard / D-pad verification\r\n\r\nKeep this window focused and follow the current instructions in chat.\r\nThe title shows the last observed key, not the active layer or all held keys.\r\nRelease all controls before the test ends. Do not unplug the device during this test.\r\nTemporary mappings end automatically.\0".encode_utf16().collect();
        let child = unsafe {
            CreateWindowExW(
                0,
                static_class.as_ptr(),
                text.as_ptr(),
                0x50000000,
                20,
                20,
                850,
                170,
                handle,
                0,
                instance,
                std::ptr::null_mut(),
            )
        };
        if child == 0 {
            return Err(win_error("Create test instructions"));
        }
        unsafe {
            SetForegroundWindow(handle);
        }
    }
    // INPUTSINK | DEVNOTIFY: no NOLEGACY, no capture or suppression flags.
    let registrations = [
        Registration {
            page: 1,
            usage: 6,
            flags: 0x2100,
            window: handle,
        },
        Registration {
            page: 1,
            usage: 2,
            flags: 0x2100,
            window: handle,
        },
        Registration {
            page: 0x0c,
            usage: 1,
            flags: 0x2100,
            window: handle,
        },
    ];
    if unsafe {
        RegisterRawInputDevices(
            registrations.as_ptr(),
            registrations.len() as u32,
            std::mem::size_of::<Registration>() as u32,
        )
    } == 0
    {
        return Err(win_error("RegisterRawInputDevices"));
    }
    let mut sources = HashMap::new();
    for device in initial_devices()? {
        if let Ok(path) = source_path(device.handle) {
            if target_path(&path) {
                writeln!(log, "# source type={} path={path}", device.kind).map_err(io_err)?;
                sources.insert(device.handle, Some(path));
            } else {
                sources.insert(device.handle, None);
            }
        }
    }
    log.flush().map_err(io_err)?;
    if unsafe { SetTimer(handle, 1, seconds * 1000 + 50, std::ptr::null()) } == 0 {
        return Err(win_error("SetTimer"));
    }
    let start = Instant::now();
    let start_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_micros();
    writeln!(log, "# start_epoch_us={start_epoch}").map_err(io_err)?;
    log.flush().map_err(io_err)?;
    let mut event_count = 0u64;
    let mut source_errors = 0u64;
    let mut message = Message::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, 0, 0, 0) };
        if result == -1 {
            return Err(win_error("GetMessageW"));
        }
        if result == 0
            || (message.message == 0x113
                && message.wparam == 1
                && message.window == handle
                && start.elapsed().as_secs() >= u64::from(seconds))
        {
            writeln!(
                log,
                "# stop reason={} elapsed_us={}",
                if result == 0 {
                    "window-closed"
                } else {
                    "deadline"
                },
                start.elapsed().as_micros()
            )
            .map_err(io_err)?;
            break;
        }
        if verify
            && matches!(message.message, 0x100 | 0x101 | 0x104 | 0x105)
            && matches!(message.wparam, 0x09 | 0x12 | 0x31 | 0x41..=0x44 | 0x25..=0x28)
        {
            let label = match message.wparam {
                0x09 => "Tab",
                0x12 => "Alt",
                0x41 => "A",
                0x42 => "B",
                0x43 => "C",
                0x44 => "D",
                0x25..=0x28 => "arrow",
                _ => "1",
            };
            let title: Vec<u16> = format!("Tartarus V2BE test - last observed key: {label}\0")
                .encode_utf16()
                .collect();
            unsafe {
                SetWindowTextW(handle, title.as_ptr());
            }
            writeln!(
                log,
                "{}\tlegacy vk={:04X} message={:04X}\ttest-window",
                start.elapsed().as_micros(),
                message.wparam,
                message.message
            )
            .map_err(io_err)?;
        }
        if message.message == 0xfe {
            let previous = sources.remove(&message.lparam).flatten();
            let path =
                previous.or_else(|| source_path(message.lparam).ok().filter(|p| target_path(p)));
            if let Some(path) = path {
                writeln!(
                    log,
                    "{}\tdevice-change={}\t{path}",
                    start.elapsed().as_micros(),
                    message.wparam
                )
                .map_err(io_err)?;
            }
        }
        if message.message == 0xff {
            // 8-byte aligned backing store; bound size to 64 KiB.
            let mut storage = [0u64; 8192];
            let mut bytes = std::mem::size_of_val(&storage) as u32;
            let read = unsafe {
                GetRawInputData(
                    message.lparam,
                    0x10000003,
                    storage.as_mut_ptr().cast(),
                    &mut bytes,
                    24,
                )
            };
            if read == u32::MAX {
                return Err(win_error("GetRawInputData"));
            }
            if read < 24 || read as usize > std::mem::size_of_val(&storage) {
                return Err("Invalid RAWINPUT length".into());
            }
            let data =
                unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), read as usize) };
            let source = isize::from_le_bytes(data[8..16].try_into().unwrap());
            if let std::collections::hash_map::Entry::Vacant(entry) = sources.entry(source) {
                match source_path(source) {
                    Ok(path) => {
                        entry.insert(target_path(&path).then_some(path));
                    }
                    Err(_) => {
                        source_errors += 1;
                    }
                }
            }
            if let Some(Some(path)) = sources.get(&source) {
                let kind = u32_at(data, 0);
                let body = &data[24..];
                let details = match kind {
                    1 if body.len() >= 16 => format!(
                        "keyboard scan={:04X} flags={:04X} vk={:04X}",
                        u16_at(body, 0),
                        u16_at(body, 2),
                        u16_at(body, 6)
                    ),
                    0 if body.len() >= 24 => format!(
                        "mouse flags={:04X} buttons={:04X} wheel={} x={} y={}",
                        u16_at(body, 0),
                        u16_at(body, 4),
                        u16_at(body, 6) as i16,
                        u32_at(body, 12) as i32,
                        u32_at(body, 16) as i32
                    ),
                    2 => format!("hid bytes={body:02X?}"),
                    _ => return Err("Malformed RAWINPUT body".into()),
                };
                writeln!(log, "{}\t{details}\t{path}", start.elapsed().as_micros())
                    .map_err(io_err)?;
                event_count += 1;
            } else if verify
                && matches!(sources.get(&source), Some(None))
                && u32_at(data, 0) == 1
                && data.len() >= 40
            {
                let body = &data[24..];
                // Only explicitly requested comparison keys; no other typing.
                if (matches!(u16_at(body, 0), 2 | 0xf | 0x38) && u16_at(body, 2) & 6 == 0)
                    || (matches!(u16_at(body, 0), 0x48 | 0x50 | 0x4b | 0x4d)
                        && u16_at(body, 2) & 6 == 2)
                {
                    writeln!(
                        log,
                        "{}\tkeyboard scan={:04X} flags={:04X} vk={:04X}\tother-keyboard",
                        start.elapsed().as_micros(),
                        u16_at(body, 0),
                        u16_at(body, 2),
                        u16_at(body, 6)
                    )
                    .map_err(io_err)?;
                    if u16_at(body, 0) == 0xf && u16_at(body, 2) & 1 == 0 {
                        writeln!(
                            log,
                            "# comparison-tab alt_down={} elapsed_us={}",
                            unsafe { GetAsyncKeyState(0x12) } < 0,
                            start.elapsed().as_micros()
                        )
                        .map_err(io_err)?;
                    }
                }
            } else if verify
                && matches!(sources.get(&source), Some(None))
                && u32_at(data, 0) == 0
                && data.len() >= 48
            {
                let body = &data[24..];
                let buttons = u16_at(body, 4) & 0xc30; // wheel axes and middle button only
                if buttons != 0 {
                    writeln!(log,"{}\tmouse flags={:04X} buttons={buttons:04X} wheel={} x=0 y=0\tother-mouse",start.elapsed().as_micros(),u16_at(body,0),u16_at(body,6) as i16).map_err(io_err)?;
                }
            }
        }
        // DefWindowProc handles required foreground WM_INPUT cleanup.
        unsafe {
            DispatchMessageW(&message);
        }
    }
    writeln!(
        log,
        "# complete events={event_count} source_lookup_errors={source_errors}"
    )
    .map_err(io_err)?;
    log.flush().map_err(io_err)?;
    drop(window);
    println!(
        "Capture complete: {event_count} target events, {source_errors} source lookup errors; {}",
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_matching_is_model_specific() {
        assert!(target_path("\\\\?\\HID#VID_1532&PID_022B&MI_02#instance"));
        assert!(!target_path("\\\\?\\HID#VID_1532&PID_0244&MI_02#instance"));
        assert!(!target_path("\\\\?\\HID#VID_1532&PID_022B0#instance"));
    }
    #[test]
    fn windows_x64_layout() {
        assert_eq!(std::mem::size_of::<Message>(), 48);
        assert_eq!(std::mem::size_of::<WindowClass>(), 72);
        assert_eq!(std::mem::size_of::<Registration>(), 16);
        assert_eq!(std::mem::size_of::<Device>(), 16);
    }
}
