use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

type Context = *mut c_void;
type Create = unsafe extern "C" fn() -> Context;
type Destroy = unsafe extern "C" fn(Context);
type HardwareId = unsafe extern "C" fn(Context, i32, *mut c_void, u32) -> u32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn SetLastError(code: u32);
}

struct Library(*mut c_void);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
impl Library {
    fn symbol(&self, name: &'static [u8]) -> Result<*mut c_void, String> {
        let ptr = unsafe { GetProcAddress(self.0, name.as_ptr()) };
        if ptr.is_null() {
            Err(format!(
                "Missing DLL export {}",
                String::from_utf8_lossy(name)
            ))
        } else {
            Ok(ptr)
        }
    }
}

struct Session {
    context: Context,
    destroy: Destroy,
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.context);
        }
    }
}

fn decode_ids(buffer: &[u16], byte_count: usize) -> Result<Vec<String>, String> {
    if byte_count > std::mem::size_of_val(buffer) || !byte_count.is_multiple_of(2) {
        return Err("Invalid hardware ID byte count returned by Interception".into());
    }
    buffer[..byte_count / 2]
        .split(|c| *c == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf16(part).map_err(|_| "Invalid UTF-16 hardware ID".into()))
        .collect()
}

fn is_v2(id: &str) -> bool {
    let upper = id.to_ascii_uppercase();
    let mut parts = upper.split(['\\', '&']);
    parts.any(|p| p == "VID_1532") && parts.any(|p| p == "PID_022B")
}

fn open(path: &Path) -> Result<(Library, Session, HardwareId), String> {
    if !cfg!(target_arch = "x86_64") {
        return Err("Use a Windows x64 build and x64 DLL".into());
    }
    if !path.is_absolute() || !path.is_file() {
        return Err("DLL must be an existing file at an absolute path; obtain the x64 DLL from the upstream release.".into());
    }
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Restrict dependency search to the selected DLL directory and System32.
    let module = unsafe { LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), 0x100 | 0x800) };
    if module.is_null() {
        return Err(format!(
            "Cannot load x64 Interception DLL: {}",
            std::io::Error::last_os_error()
        ));
    }
    let library = Library(module);
    // The signatures match the upstream C API; keep the library alive beyond Session.
    let create: Create =
        unsafe { std::mem::transmute(library.symbol(b"interception_create_context\0")?) };
    let destroy: Destroy =
        unsafe { std::mem::transmute(library.symbol(b"interception_destroy_context\0")?) };
    let hardware_id: HardwareId =
        unsafe { std::mem::transmute(library.symbol(b"interception_get_hardware_id\0")?) };
    let context = unsafe { create() };
    if context.is_null() {
        return Err("Interception context unavailable: drivers may be absent, blocked, inaccessible, or already in use. Remapping is unavailable; no fallback was enabled.".into());
    }
    let session = Session { context, destroy };
    Ok((library, session, hardware_id))
}

pub fn probe(path: &Path) -> Result<(), String> {
    let (_library, session, hardware_id) = open(path)?;
    let mut target_count = 0;
    for slot in 1..=20 {
        let mut buffer = [0u16; 4096];
        let bytes = unsafe {
            hardware_id(
                session.context,
                slot,
                buffer.as_mut_ptr().cast(),
                std::mem::size_of_val(&buffer) as u32,
            )
        };
        if bytes == 0 {
            continue;
        }
        let ids = decode_ids(&buffer, bytes as usize)?;
        if ids.iter().any(|id| is_v2(id)) {
            target_count += 1;
            println!(
                "Candidate V2 slot {slot} ({}): {}",
                if slot <= 10 { "keyboard" } else { "mouse" },
                ids.join(" | ")
            );
        }
        // Do not print unrelated hardware IDs.
    }
    if target_count == 0 {
        return Err("No V2 slots identified. This does not prove the physical device is absent. No filters were enabled.".into());
    }
    println!(
        "Found {target_count} candidate slots. Hardware IDs do not establish container identity or complete control coverage. No filters or injection enabled."
    );
    Ok(())
}

/// Keyboard/mouse prototype backend. Selection is revalidated before every delivery.
pub struct DeviceBackend {
    slot: i32,
    ids: Vec<String>,
    hardware_id: HardwareId,
    set: SetFilter,
    send: Transfer,
    receive: Transfer,
    wait: Wait,
    mouse_slot: i32,
    mouse_ids: Vec<String>,
    mouse_send: MouseTransfer,
    mouse_receive: MouseTransfer,
    session: Session,
    // Struct fields drop in declaration order: keep the DLL after the context.
    _library: Library,
}
impl DeviceBackend {
    pub fn connect(path: &Path) -> Result<Self, String> {
        let (library, session, hardware_id) = open(path)?;
        let set: SetFilter =
            unsafe { std::mem::transmute(library.symbol(b"interception_set_filter\0")?) };
        let send: Transfer =
            unsafe { std::mem::transmute(library.symbol(b"interception_send\0")?) };
        let receive: Transfer =
            unsafe { std::mem::transmute(library.symbol(b"interception_receive\0")?) };
        let wait: Wait =
            unsafe { std::mem::transmute(library.symbol(b"interception_wait_with_timeout\0")?) };
        let candidates: Vec<_> = (1..=10)
            .map(|slot| ids_for(session.context, hardware_id, slot).map(|ids| (slot, ids)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|(_, ids)| primary_v2(ids))
            .collect();
        if candidates.len() != 1 {
            return Err("Need exactly one V2 MI_00 keyboard; refusing ambiguous target".into());
        }
        let (slot, ids) = candidates.into_iter().next().unwrap();
        let mouse_candidates: Vec<_> = (11..=20)
            .map(|device| ids_for(session.context, hardware_id, device).map(|ids| (device, ids)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|(_, ids)| {
                ids.iter().any(|id| {
                    is_v2(id)
                        && id
                            .to_ascii_uppercase()
                            .split(['\\', '&'])
                            .any(|p| p == "MI_02")
                })
            })
            .collect();
        if mouse_candidates.len() != 1 {
            return Err("Need exactly one V2 MI_02 mouse path; no partial fallback enabled".into());
        }
        let (mouse_slot, mouse_ids) = mouse_candidates.into_iter().next().unwrap();
        let mouse_send: MouseTransfer =
            unsafe { std::mem::transmute(library.symbol(b"interception_send\0")?) };
        let mouse_receive: MouseTransfer =
            unsafe { std::mem::transmute(library.symbol(b"interception_receive\0")?) };
        // Taking over mid-hold could otherwise remap a repeat and consume the
        // native release of a key whose native down predates our ownership.
        if (1..=0xfe)
            .filter(|vk| *vk != 7)
            .any(|vk| unsafe { GetAsyncKeyState(vk) } < 0)
        {
            return Err(
                "Release all keyboard keys and mouse buttons before capture resumes".into(),
            );
        }
        SELECTED
            .compare_exchange(
                0,
                slot,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .map_err(|_| "Capture already active")?;
        let backend = Self {
            slot,
            ids,
            hardware_id,
            set,
            send,
            receive,
            wait,
            mouse_slot,
            mouse_ids,
            mouse_send,
            mouse_receive,
            session,
            _library: library,
        };
        unsafe {
            SetLastError(0);
            // Include extended E0/E1 keyboard strokes (the D-pad emits E0 arrows).
            // Unhandled special strokes are forwarded unchanged by the runtime.
            set(backend.session.context, selected_slot, 0xffff);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(0) {
            return Err(format!("Filter request failed: {error}"));
        }
        SELECTED_MOUSE.store(mouse_slot, std::sync::atomic::Ordering::SeqCst);
        unsafe {
            SetLastError(0);
            set(backend.session.context, selected_mouse_slot, 0xffff);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(0) {
            return Err(format!("Mouse filter request failed: {error}"));
        }
        Ok(backend)
    }
    fn validate(&self) -> Result<(), String> {
        if ids_for(self.session.context, self.hardware_id, self.slot)? != self.ids
            || ids_for(self.session.context, self.hardware_id, self.mouse_slot)? != self.mouse_ids
        {
            return Err("Target disconnected or changed identity".into());
        }
        Ok(())
    }
    pub fn next(&self, milliseconds: u32) -> Result<Option<DeviceEvent>, String> {
        let device = unsafe { (self.wait)(self.session.context, milliseconds) };
        self.validate()?;
        if device == 0 {
            return Ok(None);
        }
        if device == self.mouse_slot {
            let mut stroke = crate::mouse::MouseStroke::default();
            if unsafe { (self.mouse_receive)(self.session.context, device, &mut stroke, 1) } != 1 {
                return Err("Mouse receive failed".into());
            }
            if stroke.information == TRIAL_MARKER {
                return Err("Recursive mouse output detected".into());
            }
            return Ok(Some(DeviceEvent::Mouse(stroke)));
        }
        if device != self.slot {
            return Err("Unexpected non-target device in keyboard capture".into());
        }
        let mut stroke = KeyStroke::default();
        if unsafe { (self.receive)(self.session.context, self.slot, &mut stroke, 1) } != 1 {
            return Err("Input receive failed".into());
        }
        if stroke.information == TRIAL_MARKER {
            return Err("Recursive output detected".into());
        }
        Ok(Some(DeviceEvent::Keyboard(
            crate::mapping::Key {
                scan: stroke.code,
                extended: stroke.state & 6,
            },
            stroke.state & 1 != 0,
            stroke.information,
            stroke.state,
        )))
    }
    pub fn forward_mouse(&self, mut packet: crate::mouse::MouseStroke) -> Result<(), String> {
        self.validate()?;
        if unsafe { (self.mouse_send)(self.session.context, self.mouse_slot, &mut packet, 1) } == 1
        {
            Ok(())
        } else {
            Err("Mouse forwarding failed".into())
        }
    }
    pub fn emit(&self, output: crate::mapping::Output) -> Result<(), String> {
        self.validate()?;
        if let Some(mut packet) = crate::mouse::output_packet(output)? {
            packet.information = TRIAL_MARKER;
            return self.forward_mouse(packet);
        }
        if output.key.scan > 0x7f || !matches!(output.key.extended, 0 | 2 | 4) {
            return Err("Invalid keyboard output; refusing internal control code".into());
        }
        let mut stroke = KeyStroke {
            code: output.key.scan,
            state: output.key.extended | u16::from(output.up),
            information: TRIAL_MARKER,
        };
        if unsafe { (self.send)(self.session.context, self.slot, &mut stroke, 1) } == 1 {
            Ok(())
        } else {
            Err("Output delivery failed".into())
        }
    }
    pub fn forward_special(
        &self,
        key: crate::mapping::Key,
        state: u16,
        information: u32,
    ) -> Result<(), String> {
        self.validate()?;
        let mut stroke = KeyStroke {
            code: key.scan,
            state,
            information,
        };
        if unsafe { (self.send)(self.session.context, self.slot, &mut stroke, 1) } == 1 {
            Ok(())
        } else {
            Err("Special stroke forwarding failed".into())
        }
    }
}
impl Drop for DeviceBackend {
    fn drop(&mut self) {
        unsafe {
            (self.set)(self.session.context, selected_slot, 0);
            (self.set)(self.session.context, selected_mouse_slot, 0);
        }
        SELECTED.store(0, std::sync::atomic::Ordering::SeqCst);
        SELECTED_MOUSE.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

type Predicate = unsafe extern "C" fn(i32) -> i32;
type SetFilter = unsafe extern "C" fn(Context, Predicate, u16);
type GetFilter = unsafe extern "C" fn(Context, i32) -> u16;
type Wait = unsafe extern "C" fn(Context, u32) -> i32;
type Transfer = unsafe extern "C" fn(Context, i32, *mut KeyStroke, u32) -> i32;
type MouseTransfer = unsafe extern "C" fn(Context, i32, *mut crate::mouse::MouseStroke, u32) -> i32;
pub enum DeviceEvent {
    Keyboard(crate::mapping::Key, bool, u32, u16),
    Mouse(crate::mouse::MouseStroke),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct KeyStroke {
    code: u16,
    state: u16,
    information: u32,
}

static SELECTED: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static SELECTED_MOUSE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
unsafe extern "C" fn selected_mouse_slot(slot: i32) -> i32 {
    i32::from(slot == SELECTED_MOUSE.load(std::sync::atomic::Ordering::SeqCst))
}
unsafe extern "C" fn selected_slot(slot: i32) -> i32 {
    i32::from(slot == SELECTED.load(std::sync::atomic::Ordering::SeqCst))
}

const TRIAL_MARKER: u32 = 0x56324245;
fn translate(stroke: KeyStroke, held: &mut bool) -> KeyStroke {
    if stroke.code != 2 || stroke.state & !1 != 0 {
        return stroke;
    }
    if stroke.state == 1 && !*held {
        return stroke;
    }
    *held = stroke.state == 0;
    KeyStroke {
        code: 0x30,
        information: TRIAL_MARKER,
        ..stroke
    }
}

struct FilterGuard {
    context: Context,
    slot: i32,
    set: SetFilter,
    send: Transfer,
    held: bool,
}
impl Drop for FilterGuard {
    fn drop(&mut self) {
        // Best effort on unwinding. Normal exit performs checked cleanup below.
        if self.held {
            let mut up = KeyStroke {
                code: 0x30,
                state: 1,
                information: TRIAL_MARKER,
            };
            unsafe {
                (self.send)(self.context, self.slot, &mut up, 1);
            }
        }
        unsafe {
            (self.set)(self.context, selected_slot, 0);
        }
        SELECTED.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

fn ids_for(context: Context, hardware_id: HardwareId, slot: i32) -> Result<Vec<String>, String> {
    let mut buffer = [0u16; 4096];
    let bytes = unsafe {
        hardware_id(
            context,
            slot,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
        )
    };
    decode_ids(&buffer, bytes as usize)
}
fn primary_v2(ids: &[String]) -> bool {
    ids.iter().any(|id| {
        is_v2(id)
            && id
                .to_ascii_uppercase()
                .split(['\\', '&'])
                .any(|p| p == "MI_00")
    })
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetAsyncKeyState(key: i32) -> i16;
}

/// Explicit, bounded feasibility test, not a production capture backend.
pub fn trial(path: &Path, seconds: u32, output: &Path) -> Result<(), String> {
    use std::io::Write;
    if !(1..=60).contains(&seconds) {
        return Err("Trial duration must be 1–60 seconds".into());
    }
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let (library, session, hardware_id) = open(path)?;
    let set: SetFilter =
        unsafe { std::mem::transmute(library.symbol(b"interception_set_filter\0")?) };
    let get: GetFilter =
        unsafe { std::mem::transmute(library.symbol(b"interception_get_filter\0")?) };
    let wait: Wait =
        unsafe { std::mem::transmute(library.symbol(b"interception_wait_with_timeout\0")?) };
    let send: Transfer = unsafe { std::mem::transmute(library.symbol(b"interception_send\0")?) };
    let receive: Transfer =
        unsafe { std::mem::transmute(library.symbol(b"interception_receive\0")?) };
    let mut slots = Vec::new();
    for slot in 1..=10 {
        let ids = ids_for(session.context, hardware_id, slot)?;
        if primary_v2(&ids) {
            slots.push((slot, ids));
        }
    }
    if slots.len() != 1 {
        return Err(
            "Trial requires exactly one V2 MI_00 keyboard slot; refusing ambiguous identity".into(),
        );
    }
    let (slot, expected_ids) = slots.remove(0);
    if unsafe { GetAsyncKeyState(0x31) } < 0 || unsafe { GetAsyncKeyState(0x42) } < 0 {
        return Err("Release 1 and B on every keyboard before starting".into());
    }
    SELECTED
        .compare_exchange(
            0,
            slot,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .map_err(|_| "Another trial is active")?;
    let mut guard = FilterGuard {
        context: session.context,
        slot,
        set,
        send,
        held: false,
    };
    // Filter only ordinary down/up strokes on this one device. Other devices stay unfiltered.
    unsafe {
        SetLastError(0);
        set(session.context, selected_slot, 3);
    }
    let set_error = std::io::Error::last_os_error();
    unsafe {
        SetLastError(0);
    }
    let actual_filter = unsafe { get(session.context, slot) };
    let get_error = std::io::Error::last_os_error();
    if set_error.raw_os_error() != Some(0) || get_error.raw_os_error() != Some(0) {
        return Err(format!(
            "Filter activation could not be verified: requested=3 actual={actual_filter} set_error={set_error} get_error={get_error}"
        ));
    }
    let start_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_micros();
    writeln!(
        log,
        "# experimental slot={slot} seconds={seconds} mapping=0002->0030 filter_readback={actual_filter} start_epoch_us={start_epoch}"
    )
    .map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let mut received = 0u64;
    let mut mapped = 0u64;
    while started.elapsed().as_secs() < u64::from(seconds) {
        let device = unsafe { wait(session.context, 100) };
        if device == 0 {
            continue;
        }
        if device != slot {
            return Err("Unexpected device delivered to a single-device filter".into());
        }
        // Re-read identity before each received stroke; stale numeric slots are never trusted.
        if ids_for(session.context, hardware_id, slot)? != expected_ids {
            return Err("Target identity changed; trial aborted".into());
        }
        let mut stroke = KeyStroke::default();
        if unsafe { receive(session.context, slot, &mut stroke, 1) } != 1 {
            return Err("Interception receive failed".into());
        }
        if stroke.information == TRIAL_MARKER {
            return Err("Recursive trial input detected".into());
        }
        let was_held = guard.held;
        let replacement = translate(stroke, &mut guard.held);
        let changed = replacement != stroke;
        let mut outgoing = replacement;
        if unsafe { send(session.context, slot, &mut outgoing, 1) } != 1 {
            // If a release failed, retain ownership so the guard retries it.
            guard.held |= was_held;
            return Err("Interception delivery failed; stopping trial".into());
        }
        received += 1;
        mapped += u64::from(changed);
        // Do not record unrelated target keys or user text.
        if changed {
            writeln!(
                log,
                "{}\tmapped code={:04X} state={:04X} output={:04X}",
                started.elapsed().as_micros(),
                stroke.code,
                stroke.state,
                outgoing.code
            )
            .map_err(|e| e.to_string())?;
        }
    }
    if guard.held {
        let mut up = KeyStroke {
            code: 0x30,
            state: 1,
            information: TRIAL_MARKER,
        };
        if unsafe { send(session.context, slot, &mut up, 1) } != 1 {
            return Err("Final B release failed".into());
        }
        guard.held = false;
        writeln!(log, "# released held B at deadline").map_err(|e| e.to_string())?;
    }
    unsafe {
        set(session.context, selected_slot, 0);
    }
    if unsafe { get(session.context, slot) } != 0 {
        return Err("Filter removal could not be verified".into());
    }
    writeln!(
        log,
        "# complete received={received} mapped={mapped} filter_clear_requested=true readback=0 held=false"
    )
    .map_err(|e| e.to_string())?;
    log.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trial_mapping_ownership_and_passthrough() {
        let mut held = false;
        let key = |code, state| KeyStroke {
            code,
            state,
            information: 7,
        };
        assert_eq!(std::mem::size_of::<KeyStroke>(), 8);
        assert_eq!(translate(key(2, 1), &mut held), key(2, 1));
        assert_eq!(translate(key(2, 0), &mut held).code, 0x30);
        assert!(held);
        assert_eq!(translate(key(2, 0), &mut held).code, 0x30);
        assert_eq!(translate(key(2, 1), &mut held).code, 0x30);
        assert!(!held);
        for stroke in [key(3, 0), key(3, 1), key(2, 2), key(2, 3)] {
            assert_eq!(translate(stroke, &mut held), stroke);
        }
    }
    #[test]
    fn exact_model_matching() {
        assert!(is_v2("HID\\VID_1532&PID_022b&MI_01&Col01"));
        for id in [
            "HID\\VID_1532&PID_0244",
            "HID\\VID_1532&PID_022B0",
            "HID\\VID_15320&PID_022B",
            "HID\\VID_046D&PID_022B",
        ] {
            assert!(!is_v2(id));
        }
    }
    #[test]
    fn multi_string_and_invalid_lengths() {
        let raw: Vec<u16> = "first\0second\0\0".encode_utf16().collect();
        assert_eq!(
            decode_ids(&raw, raw.len() * 2).unwrap(),
            ["first", "second"]
        );
        assert!(decode_ids(&raw, 3).is_err());
        assert!(decode_ids(&raw, raw.len() * 2 + 2).is_err());
        assert!(decode_ids(&[0xD800], 2).is_err());
    }
}
