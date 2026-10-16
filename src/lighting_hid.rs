//! Target-only HID metadata and feature-report access; never reads input reports.
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
#[repr(C)]
struct Interface {
    size: u32,
    guid: Guid,
    flags: u32,
    reserved: usize,
}
#[repr(C)]
#[derive(Default)]
struct Attributes {
    size: u32,
    vid: u16,
    pid: u16,
    version: u16,
}
#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(guid: *mut c_void);
    fn HidD_GetAttributes(handle: isize, attrs: *mut Attributes) -> u8;
    fn HidD_GetPreparsedData(handle: isize, data: *mut *mut c_void) -> u8;
    fn HidD_FreePreparsedData(data: *mut c_void) -> u8;
    fn HidP_GetCaps(data: *const c_void, caps: *mut [u16; 32]) -> i32;
    fn HidD_SetFeature(handle: isize, report: *mut c_void, size: u32) -> u8;
    fn HidD_GetFeature(handle: isize, report: *mut c_void, size: u32) -> u8;
}
#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        guid: *const Guid,
        enumerator: *const u16,
        window: isize,
        flags: u32,
    ) -> isize;
    fn SetupDiEnumDeviceInterfaces(
        set: isize,
        device: *const c_void,
        guid: *const Guid,
        index: u32,
        data: *mut Interface,
    ) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(
        set: isize,
        data: *const Interface,
        detail: *mut c_void,
        size: u32,
        required: *mut u32,
        device: *mut c_void,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(set: isize) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const c_void,
        creation: u32,
        flags: u32,
        template: isize,
    ) -> isize;
    fn CloseHandle(handle: isize) -> i32;
    fn GetLastError() -> u32;
}
pub struct Handle(pub isize);
impl Handle {
    pub fn set_feature(&self, report: &mut [u8; 91]) -> Result<(), String> {
        if unsafe { HidD_SetFeature(self.0, report.as_mut_ptr().cast(), 91) } == 0 {
            return Err(format!(
                "Lighting feature write failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    pub fn get_feature(&self) -> Result<[u8; 91], String> {
        let mut report = [0u8; 91];
        if unsafe { HidD_GetFeature(self.0, report.as_mut_ptr().cast(), 91) } == 0 {
            return Err(format!(
                "Lighting feature read failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(report)
    }
}
pub fn control() -> Result<Handle, String> {
    let candidates: Vec<_> = enumerate()?
        .into_iter()
        .filter(|d| {
            d.usage_page == 1
                && d.usage == 2
                && d.feature_bytes == 91
                && d.path
                    .to_ascii_uppercase()
                    .split(['#', '&'])
                    .any(|s| s == "MI_02")
        })
        .collect();
    if candidates.len() != 1 {
        return Err(
            "Need exactly one Tartarus V2 MI_02 lighting interface with 91-byte feature reports"
                .into(),
        );
    }
    open(&candidates[0].path)
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct Devices(isize);
impl Drop for Devices {
    fn drop(&mut self) {
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
pub fn target(path: &str) -> bool {
    let p = path.to_ascii_uppercase();
    let parts: Vec<_> = p
        .split(|c: char| c == '#' || c == '&' || c == char::from(92))
        .collect();
    parts.contains(&"VID_1532") && parts.contains(&"PID_022B")
}
pub fn open(path: &str) -> Result<Handle, String> {
    if !target(path) {
        return Err("Refusing non-Tartarus-V2 HID path".into());
    }
    let name: Vec<_> = path.encode_utf16().chain(Some(0)).collect();
    let h = unsafe { CreateFileW(name.as_ptr(), 0, 3, ptr::null(), 3, 0, 0) };
    if h == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let handle = Handle(h);
    let mut attrs = Attributes {
        size: std::mem::size_of::<Attributes>() as u32,
        ..Default::default()
    };
    if unsafe { HidD_GetAttributes(h, &mut attrs) } == 0
        || attrs.vid != 0x1532
        || attrs.pid != 0x022b
    {
        return Err("HID identity could not be verified".into());
    }
    Ok(handle)
}
#[derive(serde::Serialize)]
pub struct Device {
    pub path: String,
    pub usage_page: u16,
    pub usage: u16,
    pub input_bytes: u16,
    pub output_bytes: u16,
    pub feature_bytes: u16,
}
pub fn enumerate() -> Result<Vec<Device>, String> {
    let mut guid = Guid::default();
    unsafe {
        HidD_GetHidGuid((&mut guid as *mut Guid).cast());
    }
    let set = unsafe { SetupDiGetClassDevsW(&guid, ptr::null(), 0, 0x12) };
    if set == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let set = Devices(set);
    let mut result = vec![];
    for index in 0..1024 {
        let mut interface = Interface {
            size: std::mem::size_of::<Interface>() as u32,
            guid: Guid::default(),
            flags: 0,
            reserved: 0,
        };
        if unsafe { SetupDiEnumDeviceInterfaces(set.0, ptr::null(), &guid, index, &mut interface) }
            == 0
        {
            if unsafe { GetLastError() } == 259 {
                break;
            }
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut needed = 0;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                ptr::null_mut(),
                0,
                &mut needed,
                ptr::null_mut(),
            );
        }
        if !(8..=65536).contains(&needed) {
            return Err("Invalid HID interface detail length".into());
        }
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        unsafe {
            ptr::write(buffer.as_mut_ptr().cast::<u32>(), 8);
        }
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                buffer.as_mut_ptr().cast(),
                needed,
                &mut needed,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let chars = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<u16>().add(2),
                (needed as usize - 4) / 2,
            )
        };
        let end = chars
            .iter()
            .position(|c| *c == 0)
            .ok_or("Unterminated HID path")?;
        let path = String::from_utf16(&chars[..end]).map_err(|e| e.to_string())?;
        if !target(&path) {
            continue;
        }
        let handle = open(&path)?;
        let mut preparsed = ptr::null_mut();
        if unsafe { HidD_GetPreparsedData(handle.0, &mut preparsed) } == 0 {
            return Err("Cannot read HID capabilities".into());
        }
        let mut caps = [0u16; 32];
        let status = unsafe { HidP_GetCaps(preparsed, &mut caps) };
        unsafe {
            HidD_FreePreparsedData(preparsed);
        }
        if status < 0 {
            return Err(format!("HID capabilities error {status:#x}"));
        }
        result.push(Device {
            path,
            usage_page: caps[1],
            usage: caps[0],
            input_bytes: caps[2],
            output_bytes: caps[3],
            feature_bytes: caps[4],
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_and_exact_v2_selection() {
        assert_eq!(std::mem::size_of::<Interface>(), 32);
        assert_eq!(std::mem::size_of::<Attributes>(), 12);
        assert_eq!(std::mem::size_of::<[u16; 32]>(), 64);
        assert!(target("HID#VID_1532&PID_022B&MI_02"));
        assert!(!target("HID#VID_1532&PID_0244&MI_02"));
        assert!(!target("HID#VID_1532&PID_022B1"));
    }
}
