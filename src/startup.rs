//! Per-user launch preferences and the single owned HKCU Run value.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "TartarusV2BE";
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        key: isize,
        sub: *const u16,
        options: u32,
        access: u32,
        out: *mut isize,
    ) -> i32;
    fn RegCreateKeyExW(
        key: isize,
        sub: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        access: u32,
        security: *const u8,
        out: *mut isize,
        disposition: *mut u32,
    ) -> i32;
    fn RegQueryValueExW(
        key: isize,
        name: *const u16,
        reserved: *mut u32,
        kind: *mut u32,
        data: *mut u8,
        len: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        key: isize,
        name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        len: u32,
    ) -> i32;
    fn RegDeleteValueW(key: isize, name: *const u16) -> i32;
    fn RegCloseKey(key: isize) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(old: *const u16, new: *const u16, flags: u32) -> i32;
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct Key(isize);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn registry(create: bool) -> Result<Option<Key>, String> {
    let mut key = 0;
    let result = if create {
        unsafe {
            RegCreateKeyExW(
                -2147483647,
                wide(KEY).as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                3,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        }
    } else {
        unsafe { RegOpenKeyExW(-2147483647, wide(KEY).as_ptr(), 0, 1, &mut key) }
    };
    if result == 2 && !create {
        return Ok(None);
    }
    if result != 0 {
        return Err(format!(
            "Cannot access current-user startup settings (Windows {result})"
        ));
    }
    Ok(Some(Key(key)))
}
fn registered() -> Result<Option<String>, String> {
    let Some(key) = registry(false)? else {
        return Ok(None);
    };
    let mut len = 0;
    let mut kind = 0;
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            wide(NAME).as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut len,
        )
    };
    if result == 2 {
        return Ok(None);
    }
    if result != 0 || kind != 1 || len > 65536 || len % 2 != 0 {
        return Err("Invalid TartarusV2BE startup registry value".into());
    }
    let mut data = vec![0u16; len as usize / 2 + 1];
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            wide(NAME).as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            data.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if result != 0 || kind != 1 {
        return Err("Startup value changed while being read".into());
    }
    data.truncate(len as usize / 2);
    while data.last() == Some(&0) {
        data.pop();
    }
    String::from_utf16(&data)
        .map(Some)
        .map_err(|e| e.to_string())
}
fn set_registered(command: Option<&str>) -> Result<(), String> {
    if command.is_none() && registered()?.is_none() {
        return Ok(());
    }
    let key = registry(true)?.unwrap();
    let result = if let Some(command) = command {
        let data = wide(command);
        unsafe {
            RegSetValueExW(
                key.0,
                wide(NAME).as_ptr(),
                0,
                1,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key.0, wide(NAME).as_ptr()) }
    };
    if result != 0 && result != 2 {
        return Err(format!(
            "Cannot update startup registration (Windows {result})"
        ));
    }
    Ok(())
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    #[serde(default = "version")]
    pub schema_version: u32,
    #[serde(default)]
    pub auto_remap: bool,
    #[serde(default)]
    pub open_browser: bool,
    #[serde(default = "enabled")]
    pub show_tray: bool,
}
fn enabled() -> bool {
    true
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            auto_remap: false,
            open_browser: false,
            show_tray: true,
        }
    }
}
fn version() -> u32 {
    1
}
impl Preferences {
    pub fn defaults() -> Self {
        Self {
            schema_version: 1,
            ..Self::default()
        }
    }
}
pub fn read(data: &Path) -> Result<Preferences, String> {
    let path = data.join("application.json");
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::defaults()),
        Err(e) => Err(e.to_string()),
    }
}
fn parse(text: &str) -> Result<Preferences, String> {
    if text.len() > 4096 {
        return Err("Application settings exceed 4 KiB".into());
    }
    let p: Preferences =
        serde_json::from_str(text).map_err(|e| format!("Invalid application.json: {e}"))?;
    if p.schema_version != 1 {
        return Err("Unsupported application settings version".into());
    }
    Ok(p)
}
fn normal(path: &Path) -> Result<String, String> {
    let value = path
        .to_str()
        .ok_or("Windows startup paths must be Unicode")?;
    let value = value.strip_prefix(r"\\?\").unwrap_or(value);
    if value.contains('"')
        || value
            .chars()
            .any(|c| c == char::from(13) || c == char::from(10))
    {
        return Err("Invalid launch path".into());
    }
    if value.starts_with(r"UNC\") {
        return Err("Network locations are not supported for startup".into());
    }
    Ok(value.to_string())
}
fn command(exe: &Path, root: &Path) -> Result<String, String> {
    let quote = |value: String| {
        let trailing = value
            .chars()
            .rev()
            .take_while(|c| *c == char::from(92))
            .count();
        format!(
            r#""{}{}""#,
            value,
            char::from(92).to_string().repeat(trailing)
        )
    };
    let value = format!(
        "{} launch --background --root {}",
        quote(normal(exe)?),
        quote(normal(root)?)
    );
    if value.encode_utf16().count() > 260 {
        return Err("Startup command exceeds Windows' 260-character Run limit; move the app to a shorter path".into());
    }
    Ok(value)
}
pub(crate) fn replace(path: &Path, text: &str) -> Result<(), String> {
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        let old: Vec<_> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let new: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(old.as_ptr(), new.as_ptr(), 9) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result
}
pub fn view(data: &Path) -> Result<serde_json::Value, String> {
    let prefs = read(data)?;
    let registration = registered()?;
    let root = data.parent().ok_or("Missing application root")?;
    let expected = serde_json::to_string(&(
        fs::read_to_string(data.join("application.json")).ok(),
        &registration,
    ))
    .map_err(|e| e.to_string())?;
    let cmd = command(&std::env::current_exe().map_err(|e| e.to_string())?, root)?;
    Ok(
        serde_json::json!({"autostart":registration.is_some(),"registration_matches":registration.as_deref()==Some(&cmd),"auto_remap":prefs.auto_remap,"open_browser":prefs.open_browser,"show_tray":prefs.show_tray,"expected":expected}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub expected: String,
    pub autostart: bool,
    pub auto_remap: bool,
    pub open_browser: bool,
    #[serde(default = "enabled")]
    pub show_tray: bool,
}
pub fn update(data: &Path, u: Update) -> Result<serde_json::Value, String> {
    if view(data)?["expected"].as_str() != Some(&u.expected) {
        return Err("Conflict: launch settings changed elsewhere; reload settings first".into());
    }
    let root = data.parent().ok_or("Missing application root")?;
    let cmd = command(&std::env::current_exe().map_err(|e| e.to_string())?, root)?;
    let previous = registered()?;
    if u.autostart && previous.as_deref().is_some_and(|v| v != cmd) {
        return Err("A different TartarusV2BE startup entry exists; disable it first before registering this location".into());
    }
    let path = data.join("application.json");
    let old = fs::read_to_string(&path).ok();
    let prefs = Preferences {
        schema_version: 1,
        auto_remap: u.auto_remap,
        open_browser: u.open_browser,
        show_tray: u.show_tray,
    };
    replace(
        &path,
        &serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?,
    )?;
    if let Err(error) = set_registered(u.autostart.then_some(cmd.as_str())) {
        let rollback = if let Some(old) = old {
            replace(&path, &old)
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())
        };
        return Err(format!("{error}; preferences rollback: {rollback:?}"));
    }
    view(data)
}
pub fn disable() -> Result<(), String> {
    set_registered(None)
}
pub fn root() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    for dir in exe.ancestors().skip(1) {
        if dir
            .join("vendor/interception/v1.0.1/Interception/library/x64/interception.dll")
            .is_file()
            || dir.join("interception.dll").is_file()
        {
            return Ok(dir.into());
        }
    }
    Err("Cannot locate interception.dll. Keep it beside the executable or use the documented project layout.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_preferences_roundtrip_and_corrupt_settings_are_rejected() {
        let dir = std::env::current_dir()
            .unwrap()
            .join("artifacts")
            .join(format!("tartarus-startup-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(read(&dir).unwrap(), Preferences::defaults());
        let path = dir.join("application.json");
        replace(
            &path,
            r#"{"schema_version":1,"auto_remap":true,"open_browser":false}"#,
        )
        .unwrap();
        assert!(read(&dir).unwrap().auto_remap);
        assert!(
            read(&dir).unwrap().show_tray,
            "Older settings keep the icon visible"
        );
        let mut hidden = read(&dir).unwrap();
        hidden.show_tray = false;
        replace(&path, &serde_json::to_string(&hidden).unwrap()).unwrap();
        assert!(!read(&dir).unwrap().show_tray);
        replace(&path, "broken").unwrap();
        assert!(read(&dir).is_err());
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn windows_argument_parser_preserves_spaces_unicode_and_root_slashes() {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn CommandLineToArgvW(command: *const u16, count: *mut i32) -> *mut *mut u16;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn LocalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }
        for root in [r"D:\", r"D:\Мой профиль\Tartarus"] {
            let cmd = command(Path::new(r"C:\App Folder\app.exe"), Path::new(root)).unwrap();
            let mut count = 0;
            let args = unsafe { CommandLineToArgvW(wide(&cmd).as_ptr(), &mut count) };
            assert!(!args.is_null());
            assert_eq!(count, 5);
            let last = unsafe { *args.add(4) };
            let mut length = 0;
            while unsafe { *last.add(length) } != 0 {
                length += 1;
            }
            assert_eq!(
                String::from_utf16(unsafe { std::slice::from_raw_parts(last, length) }).unwrap(),
                root
            );
            unsafe {
                LocalFree(args.cast());
            }
        }
    }
    #[test]
    fn safe_commands_and_strict_settings() {
        assert_eq!(
            command(Path::new(r"C:\App Folder\app.exe"), Path::new(r"D:\My App")).unwrap(),
            r#""C:\App Folder\app.exe" launch --background --root "D:\My App""#
        );
        assert!(command(Path::new(r#"bad"path"#), Path::new("x")).is_err());
        assert!(command(Path::new(&"x".repeat(260)), Path::new("x")).is_err());
        assert!(parse(r#"{"schema_version":2}"#).is_err());
        assert!(parse(r#"{"schema_version":1,"typo":true}"#).is_err());
        assert_eq!(parse("{}").unwrap(), Preferences::defaults());
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(window: isize, text: *const u16, title: *const u16, kind: u32) -> i32;
}
pub fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            0,
            wide(message).as_ptr(),
            wide("Tartarus V2BE").as_ptr(),
            0x10,
        );
    }
}
fn existing() -> Result<bool, String> {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
    };
    let address = "127.0.0.1:17832"
        .parse()
        .map_err(|_| "Invalid editor address")?;
    // A free port needs no TCP connection attempt (Windows can defer refusing
    // a connection to a closed local port beyond a short connection timeout).
    match std::net::TcpListener::bind(address) {
        Ok(listener) => {
            drop(listener);
            return Ok(false);
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {}
        Err(e) => return Err(format!("Cannot reserve editor port: {e}")),
    }
    let mut stream = match TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => return Ok(false),
        Err(e) => return Err(format!("Cannot check existing editor: {e}")),
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1:17832\r\nConnection: close\r\n\r\n")
        .map_err(|e| e.to_string())?;
    let mut body = String::new();
    stream
        .take(65537)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    if body.starts_with("HTTP/1.1 200 ")
        && body.contains("<title>Tartarus V2BE</title>")
        && body.contains(r#"name="editor-token""#)
    {
        Ok(true)
    } else {
        Err("Port 17832 is occupied by another server; refusing to start another instance".into())
    }
}
pub fn launch(root: PathBuf, background: bool, force_open: bool) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let data = root.join("data");
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let prefs = read(&data)?;
    let open = !background && (force_open || prefs.open_browser);
    if existing()? {
        // Manual relaunch remains a way back into an app whose tray is hidden.
        if !background {
            crate::tray::open(0)?;
        }
        return Ok(());
    }
    let dll = if root.join("interception.dll").is_file() {
        root.join("interception.dll")
    } else {
        root.join("vendor/interception/v1.0.1/Interception/library/x64/interception.dll")
    };
    if !dll.is_file() {
        return Err("Interception DLL is missing; no driver was installed or changed".into());
    }
    let config = data.join("config.json");
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config)
    {
        Ok(mut file) => file
            .write_all(br#"{"schema_version":1,"mappings":{}}"#)
            .map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    crate::web::serve(dll, config, data.join("logs"), open)
}
