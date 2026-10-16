//! Native notification icon; one blocking message-loop thread, no polling.
use crate::runtime;
use std::{
    cell::RefCell,
    ffi::c_void,
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc,
    thread::JoinHandle,
    time::Duration,
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
struct Notify {
    size: u32,
    window: isize,
    id: u32,
    flags: u32,
    callback: u32,
    icon: isize,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    version: u32,
    title: [u16; 64],
    info_flags: u32,
    guid: [u8; 16],
    balloon: isize,
}
#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn Shell_NotifyIconW(message: u32, data: *const Notify) -> i32;
    fn ShellExecuteW(
        window: isize,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const WindowClass) -> u16;
    fn UnregisterClassW(name: *const u16, instance: isize) -> i32;
    fn RegisterWindowMessageW(name: *const u16) -> u32;
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
    fn PostMessageW(window: isize, message: u32, wparam: usize, lparam: isize) -> i32;
    fn PostQuitMessage(code: i32);
    fn CreateIcon(
        instance: isize,
        width: i32,
        height: i32,
        planes: u8,
        bits: u8,
        and: *const u8,
        xor: *const u8,
    ) -> isize;
    fn DestroyIcon(icon: isize) -> i32;
    fn CreatePopupMenu() -> isize;
    fn AppendMenuW(menu: isize, flags: u32, id: usize, label: *const u16) -> i32;
    fn DestroyMenu(menu: isize) -> i32;
    fn TrackPopupMenu(
        menu: isize,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        window: isize,
        rect: *const c_void,
    ) -> i32;
    fn SetForegroundWindow(window: isize) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn MessageBoxW(window: isize, text: *const u16, caption: *const u16, kind: u32) -> i32;
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
const CALLBACK: u32 = 0x8001;
#[derive(Clone)]
struct State {
    token: String,
    status: runtime::SharedStatus,
    icon: isize,
    taskbar: u32,
}
thread_local! {static STATE:RefCell<Option<State>>=const{RefCell::new(None)};}
fn notify(window: isize, icon: isize) -> Notify {
    let mut data: Notify = unsafe { std::mem::zeroed() };
    data.size = std::mem::size_of::<Notify>() as u32;
    data.window = window;
    data.id = 1;
    data.flags = 1 | 2 | 4;
    data.callback = CALLBACK;
    data.icon = icon;
    for (dst, src) in data
        .tip
        .iter_mut()
        .zip("Tartarus V2BE — right-click for controls".encode_utf16())
    {
        *dst = src;
    }
    data
}
fn api(token: &str, path: &str) -> Result<(), String> {
    let address = "127.0.0.1:17832"
        .parse()
        .map_err(|_| "Invalid local editor address")?;
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    write!(stream,"POST {path} HTTP/1.1\r\nHost: 127.0.0.1:17832\r\nX-Tartarus-Token: {token}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").map_err(|e|e.to_string())?;
    let mut response = String::new();
    stream
        .take(16385)
        .read_to_string(&mut response)
        .map_err(|e| e.to_string())?;
    if response.starts_with("HTTP/1.1 200 ") {
        return Ok(());
    }
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .unwrap_or("");
    let reason = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"].as_str().map(String::from));
    Err(reason.unwrap_or_else(|| "Local editor did not accept the tray action".into()))
}
pub fn open(window: isize) -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            window,
            wide("open").as_ptr(),
            wide("http://127.0.0.1:17832/").as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        Err(format!(
            "Could not open the configuration browser (Windows code {result})"
        ))
    } else {
        Ok(())
    }
}
fn error(window: isize, error: String) {
    unsafe {
        MessageBoxW(
            window,
            wide(&error).as_ptr(),
            wide("Tartarus V2BE").as_ptr(),
            0x10,
        );
    }
}
fn menu(window: isize, state: &State) {
    let handle = unsafe { CreatePopupMenu() };
    if handle == 0 {
        return;
    }
    let observed = state.status.lock().unwrap_or_else(|e| e.into_inner());
    let running = observed.running || observed.recovering.is_some();
    drop(observed);
    let entries = [
        (0, 1, "Open configuration"),
        (if running { 1 } else { 0 }, 2, "Start remapping"),
        (if running { 0 } else { 1 }, 3, "Stop remapping"),
        (0x800, 0, ""),
        (0, 4, "Exit"),
    ];
    for (flags, id, label) in entries {
        if unsafe { AppendMenuW(handle, flags, id, wide(label).as_ptr()) } == 0 {
            unsafe {
                DestroyMenu(handle);
            };
            return;
        }
    }
    let mut point = Point::default();
    unsafe {
        GetCursorPos(&mut point);
        SetForegroundWindow(window);
    }
    let command = unsafe {
        TrackPopupMenu(
            handle,
            0x100 | 0x2,
            point.x,
            point.y,
            0,
            window,
            std::ptr::null(),
        )
    };
    unsafe {
        PostMessageW(window, 0, 0, 0);
        DestroyMenu(handle);
    }
    let result = match command {
        1 => open(window),
        2 => api(&state.token, "/api/start"),
        3 => api(&state.token, "/api/stop"),
        4 => api(&state.token, "/api/tray/exit"),
        _ => Ok(()),
    };
    if let Err(e) = result {
        error(window, e);
    }
}
unsafe extern "system" fn window_proc(
    window: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let state = STATE.with(|s| s.borrow().clone());
    if let Some(state) = state {
        if message == state.taskbar {
            unsafe {
                Shell_NotifyIconW(0, &notify(window, state.icon));
            }
            return 0;
        }
        if message == CALLBACK {
            match lparam as u32 {
                0x203 | 0x400 | 0x401 => {
                    if let Err(e) = open(window) {
                        error(window, e);
                    }
                }
                0x205 | 0x7b => menu(window, &state),
                _ => {}
            }
            return 0;
        }
    }
    if message == 0x10 {
        unsafe {
            DestroyWindow(window);
        }
        return 0;
    }
    if message == 2 {
        unsafe {
            PostQuitMessage(0);
        }
        return 0;
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}
fn icon(instance: isize) -> isize {
    let mask = [0u8; 128];
    let mut pixels = [0u8; 4096];
    for y in 0..32 {
        for x in 0..32 {
            let i = (y * 32 + x) * 4;
            let t = (7..25).contains(&x) && (7..12).contains(&y)
                || (13..19).contains(&x) && (10..26).contains(&y);
            let color = if t {
                [16, 32, 20, 255]
            } else {
                [119, 244, 181, 255]
            };
            pixels[i..i + 4].copy_from_slice(&color);
        }
    }
    unsafe { CreateIcon(instance, 32, 32, 1, 32, mask.as_ptr(), pixels.as_ptr()) }
}
pub struct Tray {
    window: isize,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.window, 0x10, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn spawn(token: String, status: runtime::SharedStatus) -> Result<Tray, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class = wide("TartarusV2BE.Tray");
        let wc = WindowClass {
            style: 0,
            proc: window_proc,
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: 0,
            cursor: 0,
            brush: 0,
            menu: std::ptr::null(),
            name: class.as_ptr(),
        };
        if unsafe { RegisterClassW(&wc) } == 0 {
            let _ = tx.send(Err("Cannot register tray window".into()));
            return;
        }
        let taskbar = unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) };
        let icon = icon(instance);
        if taskbar == 0 || icon == 0 {
            unsafe {
                if icon != 0 {
                    DestroyIcon(icon);
                }
                UnregisterClassW(class.as_ptr(), instance);
            }
            let _ = tx.send(Err("Cannot create tray resources".into()));
            return;
        }
        STATE.with(|s| {
            *s.borrow_mut() = Some(State {
                token,
                status,
                icon,
                taskbar,
            })
        });
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                wide("Tartarus V2BE tray").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                instance,
                std::ptr::null_mut(),
            )
        };
        if window == 0 || unsafe { Shell_NotifyIconW(0, &notify(window, icon)) } == 0 {
            unsafe {
                if window != 0 {
                    DestroyWindow(window);
                }
                DestroyIcon(icon);
                UnregisterClassW(class.as_ptr(), instance);
            }
            STATE.with(|s| *s.borrow_mut() = None);
            let _ = tx.send(Err("Windows could not add the Tartarus tray icon".into()));
            return;
        }
        if tx.send(Ok(window)).is_ok() {
            let mut message = Message::default();
            while unsafe { GetMessageW(&mut message, 0, 0, 0) } > 0 {
                unsafe {
                    DispatchMessageW(&message);
                }
            }
        }
        unsafe {
            Shell_NotifyIconW(2, &notify(window, icon));
            DestroyWindow(window);
            DestroyIcon(icon);
            UnregisterClassW(class.as_ptr(), instance);
        }
        STATE.with(|s| *s.borrow_mut() = None);
    });
    match rx.recv() {
        Ok(Ok(window)) => Ok(Tray {
            window,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(e) => {
            let _ = thread.join();
            Err(e.to_string())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires the interactive Windows Explorer notification area"]
    fn explorer_icon_lifecycle() {
        #[repr(C)]
        struct Identifier {
            size: u32,
            window: isize,
            id: u32,
            guid: [u8; 16],
        }
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn Shell_NotifyIconGetRect(id: *const Identifier, rect: *mut [i32; 4]) -> i32;
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn IsWindow(window: isize) -> i32;
        }
        let tray = spawn(
            "lifecycle-test-not-an-api-token".into(),
            runtime::SharedStatus::default(),
        )
        .unwrap();
        let id = Identifier {
            size: std::mem::size_of::<Identifier>() as u32,
            window: tray.window,
            id: 1,
            guid: [0; 16],
        };
        let mut rect = [0i32; 4];
        assert!(unsafe { Shell_NotifyIconGetRect(&id, &mut rect) } >= 0);
        assert_ne!(unsafe { IsWindow(id.window) }, 0);
        drop(tray);
        assert_eq!(unsafe { IsWindow(id.window) }, 0);
        assert!(unsafe { Shell_NotifyIconGetRect(&id, &mut rect) } < 0);
    }
    #[test]
    fn windows_layout_and_null_terminated_tooltip() {
        assert_eq!(std::mem::size_of::<Notify>(), 976);
        assert_eq!(std::mem::size_of::<Message>(), 48);
        let n = notify(42, 99);
        assert_eq!(n.window, 42);
        assert_eq!(n.icon, 99);
        assert_eq!(n.tip[127], 0);
    }
}
