//! Native PnP/power callbacks only signal a bounded wake event.
use std::{
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
#[repr(C)]
struct Filter {
    size: u32,
    flags: u32,
    kind: u32,
    reserved: u32,
    data: [u64; 50],
}
#[repr(C)]
struct Subscribe {
    callback: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> u32,
    context: *mut c_void,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateEventW(a: *const c_void, manual: i32, initial: i32, name: *const u16) -> isize;
    fn SetEvent(h: isize) -> i32;
    fn ResetEvent(h: isize) -> i32;
    fn CloseHandle(h: isize) -> i32;
    fn WaitForSingleObject(h: isize, ms: u32) -> u32;
    fn WaitForMultipleObjects(n: u32, handles: *const isize, all: i32, ms: u32) -> u32;
}
#[link(name = "cfgmgr32")]
unsafe extern "system" {
    fn CM_Register_Notification(
        filter: *const Filter,
        context: *mut c_void,
        callback: unsafe extern "system" fn(isize, *mut c_void, u32, *const u8, u32) -> u32,
        out: *mut isize,
    ) -> u32;
    fn CM_Unregister_Notification(handle: isize) -> u32;
}
#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(guid: *mut c_void);
}
#[link(name = "powrprof")]
unsafe extern "system" {
    fn PowerRegisterSuspendResumeNotification(
        flags: u32,
        recipient: *const Subscribe,
        out: *mut isize,
    ) -> u32;
    fn PowerUnregisterSuspendResumeNotification(handle: isize) -> u32;
}
struct State {
    event: isize,
    ack: isize,
    generation: AtomicU64,
    suspended: AtomicBool,
    wait_for_cleanup: bool,
}
impl State {
    fn signal(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        unsafe {
            SetEvent(self.event);
        }
    }
}
pub struct Monitor {
    state: Box<State>,
    device: isize,
    power: isize,
    recipient: Box<Subscribe>,
}
fn target(path: &str) -> bool {
    let path = path.to_ascii_uppercase();
    let parts: Vec<_> = path
        .split(|c: char| c == '#' || c == '&' || c == char::from(92))
        .collect();
    parts.contains(&"VID_1532") && parts.contains(&"PID_022B")
}
unsafe extern "system" fn device(
    _handle: isize,
    context: *mut c_void,
    action: u32,
    data: *const u8,
    size: u32,
) -> u32 {
    if context.is_null()
        || data.is_null()
        || !matches!(action, 0 | 1)
        || !(26..=65536).contains(&size)
    {
        return 0;
    }
    // CM_NOTIFY_EVENT_DATA: type/reserved (8), interface GUID (16), UTF-16 link.
    let bytes = unsafe { std::slice::from_raw_parts(data, size as usize) };
    if u32::from_ne_bytes(bytes[..4].try_into().unwrap()) != 0 {
        return 0;
    }
    let chars: Vec<_> = bytes[24..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_ne_bytes([c[0], c[1]]))
        .take_while(|c| *c != 0)
        .collect();
    if target(&String::from_utf16_lossy(&chars)) {
        unsafe { &*(context as *const State) }.signal();
    }
    0
}
unsafe extern "system" fn power(context: *mut c_void, kind: u32, _setting: *mut c_void) -> u32 {
    let state = unsafe { &*(context as *const State) };
    match kind {
        4 => {
            unsafe {
                ResetEvent(state.ack);
            }
            state.suspended.store(true, Ordering::SeqCst);
            state.signal();
            if state.wait_for_cleanup {
                unsafe {
                    WaitForSingleObject(state.ack, 1000);
                }
            }
        }
        7 | 18 => {
            state.suspended.store(false, Ordering::SeqCst);
            state.signal();
        }
        _ => {}
    }
    0
}
impl Monitor {
    pub fn new() -> Result<Self, String> {
        Self::create(true)
    }
    pub fn lighting() -> Result<Self, String> {
        Self::create(false)
    }
    pub fn wake_handle(&self) -> isize {
        self.state.event
    }
    fn create(wait_for_cleanup: bool) -> Result<Self, String> {
        let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
        let ack = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        if event == 0 || ack == 0 {
            unsafe {
                if event != 0 {
                    CloseHandle(event);
                }
                if ack != 0 {
                    CloseHandle(ack);
                }
            }
            return Err("Cannot create recovery events".into());
        }
        let mut monitor = Self {
            state: Box::new(State {
                event,
                ack,
                generation: AtomicU64::new(0),
                suspended: AtomicBool::new(false),
                wait_for_cleanup,
            }),
            device: 0,
            power: 0,
            recipient: Box::new(Subscribe {
                callback: power,
                context: std::ptr::null_mut(),
            }),
        };
        let context = (&mut *monitor.state) as *mut State as *mut c_void;
        let mut filter = Filter {
            size: std::mem::size_of::<Filter>() as u32,
            flags: 0,
            kind: 0,
            reserved: 0,
            data: [0; 50],
        };
        unsafe {
            HidD_GetHidGuid(filter.data.as_mut_ptr().cast());
        }
        let result =
            unsafe { CM_Register_Notification(&filter, context, device, &mut monitor.device) };
        if result != 0 {
            return Err(format!(
                "Cannot register device recovery notifications: {result}"
            ));
        }
        monitor.recipient.context = context;
        let result = unsafe {
            PowerRegisterSuspendResumeNotification(2, &*monitor.recipient, &mut monitor.power)
        };
        if result != 0 {
            return Err(format!(
                "Cannot register sleep recovery notifications: {result}"
            ));
        }
        Ok(monitor)
    }
    pub fn generation(&self) -> u64 {
        self.state.generation.load(Ordering::SeqCst)
    }
    pub fn suspended(&self) -> bool {
        self.state.suspended.load(Ordering::SeqCst)
    }
    pub fn acknowledge(&self) {
        unsafe {
            SetEvent(self.state.ack);
        }
    }
    pub fn wait(&self, stop: isize, ms: u32) -> Result<(), String> {
        let handles = [stop, self.state.event];
        match unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, ms) } {
            0 | 1 | 258 => Ok(()),
            _ => Err("Recovery wait failed".into()),
        }
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        // Unregister waits for callbacks before their boxed context and handles drop.
        self.acknowledge();
        unsafe {
            if self.power != 0 {
                PowerUnregisterSuspendResumeNotification(self.power);
            }
            if self.device != 0 {
                CM_Unregister_Notification(self.device);
            }
            CloseHandle(self.state.event);
            CloseHandle(self.state.ack);
        }
    }
}
pub fn retry_delay(attempt: u32) -> u32 {
    if attempt < 10 { 1000 } else { 30000 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_registration_lifetime_and_target_only_wakes() {
        let mut monitor = Monitor::new().unwrap();
        let context = (&mut *monitor.state) as *mut State as *mut c_void;
        let mut bytes = vec![0u8; 24];
        bytes.extend(
            r"HID#VID_1532&PID_0244#other"
                .encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_ne_bytes),
        );
        unsafe {
            device(0, context, 0, bytes.as_ptr(), bytes.len() as u32);
        }
        assert_eq!(monitor.generation(), 0);
        bytes.truncate(24);
        bytes.extend(
            r"HID#VID_1532&PID_022B&MI_00#target"
                .encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_ne_bytes),
        );
        unsafe {
            device(0, context, 1, bytes.as_ptr(), bytes.len() as u32);
        }
        assert_eq!(monitor.generation(), 1);
        monitor.state.suspended.store(true, Ordering::SeqCst);
        unsafe {
            power(context, 18, std::ptr::null_mut());
        }
        assert!(!monitor.suspended());
        assert_eq!(monitor.generation(), 2);
        monitor.acknowledge();
    }
    #[test]
    fn identities_and_native_layouts() {
        assert_eq!(std::mem::size_of::<Filter>(), 416);
        assert_eq!(std::mem::size_of::<Subscribe>(), 16);
        assert!(target(r"\?HID#VID_1532&PID_022b&MI_00#abc"));
        assert!(!target(r"\?HID#VID_1532&PID_0244#abc"));
        assert!(!target(r"\?HID#VID_1532&PID_022B1#abc"));
        assert_eq!(retry_delay(0), 1000);
        assert_eq!(retry_delay(100), 30000);
    }
}
