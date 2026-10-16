//! Small, bounded localhost editor. No global hooks, external assets, or driver installation.
use crate::{config, runtime};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread::JoinHandle,
    time::Duration,
};
const ADDRESS: &str = "127.0.0.1:17832";
const MAX_BODY: usize = 150_000;
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: isize, buffer: *mut u8, length: u32, flags: u32) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(old: *const u16, new: *const u16, flags: u32) -> i32;
}

struct Request {
    method: String,
    path: String,
    host: String,
    origin: Option<String>,
    token: Option<String>,
    body: String,
}
fn request(stream: &mut impl Read) -> Result<Request, String> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= 8192 {
            return Err("Headers too large".into());
        }
        let mut byte = [0];
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        bytes.push(byte[0]);
    }
    let header = std::str::from_utf8(&bytes).map_err(|_| "Invalid headers")?;
    let mut lines = header.split("\r\n");
    let parts: Vec<_> = lines.next().unwrap_or_default().split(' ').collect();
    if parts.len() != 3 || parts[2] != "HTTP/1.1" {
        return Err("Expected HTTP/1.1".into());
    }
    let mut headers = std::collections::BTreeMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (key, value) = line.split_once(':').ok_or("Malformed header")?;
        if headers
            .insert(key.to_ascii_lowercase(), value.trim().to_string())
            .is_some()
        {
            return Err("Duplicate header".into());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err("Chunked requests are not supported".into());
    }
    let length = headers
        .get("content-length")
        .map(|s| s.parse::<usize>().map_err(|_| "Invalid length"))
        .transpose()?
        .unwrap_or(0);
    if length > MAX_BODY {
        return Err("Request too large".into());
    }
    if parts[0] == "POST"
        && !headers
            .get("content-type")
            .is_some_and(|s| s.split(';').next() == Some("application/json"))
    {
        return Err("Expected application/json".into());
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok(Request {
        method: parts[0].into(),
        path: parts[1].into(),
        host: headers.remove("host").unwrap_or_default(),
        origin: headers.remove("origin"),
        token: headers.remove("x-tartarus-token"),
        body: String::from_utf8(body).map_err(|_| "Expected UTF-8")?,
    })
}
fn authorized(r: &Request, token: &str) -> bool {
    r.host == ADDRESS
        && r.origin
            .as_deref()
            .is_none_or(|o| o == "http://127.0.0.1:17832")
        && (!r.path.starts_with("/api/") || r.token.as_deref() == Some(token))
}
fn respond(stream: &mut TcpStream, code: u16, mime: &str, body: &str) {
    respond_bytes(stream, code, mime, body.as_bytes());
}
fn respond_bytes(stream: &mut TcpStream, code: u16, mime: &str, body: &[u8]) {
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        429 => "Too Many Requests",
        _ => "Internal Server Error",
    };
    let cache = if mime == "font/woff2" {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: {cache}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; connect-src 'self'; img-src 'self' blob:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}
// Same-directory replacement lets the runtime observe only a complete validated file.
fn save(path: &Path, expected: &str, text: &str, token: &str) -> Result<(), String> {
    config::parse(text)?;
    let current = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if current != expected {
        return Err("Conflict: configuration changed elsewhere. Reload before saving.".into());
    }
    let temp = path.with_extension(format!("{token}.tmp"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        use std::os::windows::ffi::OsStrExt;
        let old: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let new: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(old.as_ptr(), new.as_ptr(), 1 | 8) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    expected: String,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileSave {
    name: String,
    text: String,
    expected: Option<String>,
}
struct App {
    tray: Option<crate::tray::Tray>,
    token: String,
    lighting: Result<crate::lighting::Controller, String>,
    dll: PathBuf,
    config: PathBuf,
    logs: PathBuf,
    status: runtime::SharedStatus,
    worker: Option<JoinHandle<Result<(), String>>>,
    last_error: Option<String>,
    log: Option<PathBuf>,
}
impl App {
    fn start(&mut self, seconds: Option<u32>) -> Result<(), String> {
        if seconds.is_some_and(|s| !(1..=300).contains(&s)) {
            return Err("Test duration must be 1–300 seconds".into());
        }
        if self.worker.is_some() {
            return Err("Runtime is already starting or running".into());
        }
        config::load(&self.config)?;
        let dll = self.dll.clone();
        let path = self.config.clone();
        let status = self.status.clone();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let log = self.logs.join(format!("ui-{stamp}.log"));
        self.log = Some(log.clone());
        self.last_error = None;
        self.worker = Some(std::thread::spawn(move || {
            runtime::run_monitored(&dll, &path, &log, seconds, &status)
        }));
        Ok(())
    }
    fn reap(&mut self) {
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            {
                let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
                s.running = false;
                s.recovering = None;
                s.live_frame = crate::live::Frame::default();
                s.live.publish(&s.live_frame);
            }
            self.last_error = self
                .worker
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|_| Err("Runtime thread stopped unexpectedly".into()))
                .err();
        }
    }
    fn api(&mut self, r: &Request, token: &str) -> Result<serde_json::Value, String> {
        self.reap();
        match (r.method.as_str(), r.path.as_str()) {
            ("GET", "/api/lighting") => self.lighting.as_ref().map_err(Clone::clone)?.view(),
            ("POST", "/api/lighting") => {
                let update = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                self.lighting.as_ref().map_err(Clone::clone)?.save(update)
            }
            ("GET", "/api/settings") => {
                crate::startup::view(self.config.parent().ok_or("Missing settings directory")?)
            }
            ("POST", "/api/settings") => {
                let update: crate::startup::Update =
                    serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                if update.autostart {
                    let root = self
                        .config
                        .parent()
                        .and_then(|p| p.parent())
                        .ok_or("Missing application root")?;
                    if fs::canonicalize(root.join("data/config.json"))
                        .ok()
                        .as_ref()
                        != Some(&self.config)
                    {
                        return Err(
                            "Autostart requires the standard data/config.json layout".into()
                        );
                    }
                    let dll = if root.join("interception.dll").is_file() {
                        root.join("interception.dll")
                    } else {
                        root.join(
                            "vendor/interception/v1.0.1/Interception/library/x64/interception.dll",
                        )
                    };
                    if fs::canonicalize(dll).ok().as_ref() != Some(&self.dll) {
                        return Err(
                            "Autostart requires the standard Interception DLL location".into()
                        );
                    }
                }
                let new_tray = if update.show_tray && self.tray.is_none() {
                    Some(crate::tray::spawn(self.token.clone(), self.status.clone())?)
                } else {
                    None
                };
                let show_tray = update.show_tray;
                let result = crate::startup::update(
                    self.config.parent().ok_or("Missing settings directory")?,
                    update,
                )?;
                if !show_tray {
                    self.tray.take();
                } else if let Some(tray) = new_tray {
                    self.tray = Some(tray);
                }
                Ok(result)
            }
            ("POST", "/api/synapse/preview") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Upload {
                    name: String,
                    bytes: Vec<u8>,
                }
                let upload: Upload = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                serde_json::to_value(crate::synapse::preview(&upload.name, &upload.bytes)?)
                    .map_err(|e| e.to_string())
            }
            ("POST", "/api/validate") => {
                #[derive(Deserialize)]
                struct Text {
                    text: String,
                }
                let body: Text = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                config::parse(&body.text)?;
                Ok(json!({"valid":true}))
            }
            ("GET", "/api/profiles") => {
                let dir = self
                    .config
                    .parent()
                    .ok_or("Missing config directory")?
                    .join("profiles");
                let mut profiles = Vec::new();
                if dir.exists() {
                    for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.take(65) {
                        let entry = entry.map_err(|e| e.to_string())?;
                        if entry.path().extension().is_some_and(|e| e == "json") {
                            let (text, _) = config::load(&entry.path())?;
                            profiles.push(json!({"name":entry.path().file_stem().unwrap_or_default().to_string_lossy(),"text":text}));
                        }
                    }
                }
                Ok(json!({"profiles":profiles}))
            }
            ("POST", "/api/profile") => {
                let body: ProfileSave = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                if body.name.is_empty()
                    || body.name.len() > 64
                    || !body
                        .name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
                {
                    return Err(
                        "Profile names use 1–64 letters, numbers, underscores or hyphens".into(),
                    );
                }
                let upper = body.name.to_ascii_uppercase();
                if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || (upper.len() == 4
                        && (upper.starts_with("COM") || upper.starts_with("LPT"))
                        && matches!(upper.as_bytes()[3], b'1'..=b'9'))
                {
                    return Err("Reserved Windows profile name".into());
                }
                config::parse(&body.text)?;
                let dir = self
                    .config
                    .parent()
                    .ok_or("Missing config directory")?
                    .join("profiles");
                fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(format!("{}.json", body.name));
                if path.exists() {
                    save(
                        &path,
                        body.expected
                            .as_deref()
                            .ok_or("Profile already exists; select it before updating")?,
                        &body.text,
                        token,
                    )?;
                } else {
                    if fs::read_dir(&dir).map_err(|e| e.to_string())?.count() >= 64 {
                        return Err("Profile library is limited to 64 entries".into());
                    }
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)
                        .map_err(|e| e.to_string())?;
                    file.write_all(body.text.as_bytes())
                        .and_then(|_| file.sync_all())
                        .map_err(|e| e.to_string())?;
                }
                Ok(json!({"saved":true}))
            }
            ("GET", "/api/config") => {
                let (text, _) = config::load(&self.config)?;
                Ok(json!({"text":text}))
            }
            ("POST", "/api/config") => {
                let body: Save = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                save(&self.config, &body.expected, &body.text, token)?;
                Ok(json!({"saved":true}))
            }
            ("GET", "/api/status") => {
                let s = self.status.lock().unwrap_or_else(|e| e.into_inner());
                let disk = fs::read_to_string(&self.config).ok();
                let lighting = match &self.lighting {
                    Ok(lighting) => lighting.observation(),
                    Err(e) => json!({"status":"Lighting unavailable","error":e}),
                };
                Ok(
                    json!({"lighting":lighting,"running":s.running,"starting":self.worker.is_some()&&!s.running,"recovering":s.recovering,"applied":s.running&&disk.as_ref()==Some(&s.applied_text),"layer":s.layer,"live_viewers":s.live.viewers(),"error":self.last_error,"log":self.log,"device":if s.running {"Connected; target interfaces captured"} else {"Not captured"},"driver":if s.running {"Interception active"} else {"Not capturing"}}),
                )
            }
            ("POST", "/api/start") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Start {
                    seconds: Option<u32>,
                }
                let start: Start = serde_json::from_str(&r.body).map_err(|e| e.to_string())?;
                self.start(start.seconds)?;
                Ok(json!({"starting":true}))
            }
            ("POST", "/api/stop") => {
                if self.worker.is_none() {
                    return Ok(json!({"stopped":true}));
                }
                runtime::stop()?;
                Ok(json!({"stopping":true}))
            }
            ("POST", "/api/exit") => {
                if self.worker.is_some() {
                    return Err("Stop remapping and wait for it to finish before exiting".into());
                }
                Ok(json!({"exiting":true}))
            }
            ("POST", "/api/tray/exit") => {
                if self.worker.is_some() {
                    runtime::stop()?;
                    self.worker
                        .take()
                        .unwrap()
                        .join()
                        .map_err(|_| "Runtime thread failed during shutdown")??;
                }
                Ok(json!({"exiting":true}))
            }
            _ => Err("Unknown endpoint".into()),
        }
    }
}
pub fn serve(dll: PathBuf, path: PathBuf, logs: PathBuf, open_browser: bool) -> Result<(), String> {
    let config = fs::canonicalize(path).map_err(|e| e.to_string())?;
    config::load(&config)?;
    let dll = fs::canonicalize(dll).map_err(|e| e.to_string())?;
    fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    let logs = fs::canonicalize(logs).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind(ADDRESS)
        .map_err(|e| format!("Cannot open local editor at {ADDRESS}: {e}"))?;
    let mut random = [0u8; 32];
    if unsafe { BCryptGenRandom(0, random.as_mut_ptr(), 32, 2) } < 0 {
        return Err("Cannot generate editor token".into());
    }
    let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let html = include_str!("../web/index.html").replace("__TOKEN__", &token);
    let lighting = crate::lighting::Controller::new(
        config
            .parent()
            .ok_or("Missing lighting settings directory")?,
    );
    let mut app = App {
        tray: None,
        token: token.clone(),
        lighting,
        dll,
        config,
        logs,
        status: runtime::SharedStatus::default(),
        worker: None,
        last_error: None,
        log: None,
    };
    if crate::startup::read(app.config.parent().ok_or("Missing settings directory")?)?.show_tray {
        app.tray = Some(crate::tray::spawn(token.clone(), app.status.clone())?);
    }
    let _live_close = crate::live::Close(
        app.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .live
            .clone(),
    );
    match crate::startup::read(app.config.parent().ok_or("Missing settings directory")?) {
        Ok(prefs) if prefs.auto_remap => {
            if let Err(e) = app.start(None) {
                app.last_error = Some(e);
            }
        }
        Ok(_) => {}
        Err(e) => app.last_error = Some(e),
    }
    if open_browser && let Err(e) = crate::tray::open(0) {
        eprintln!("{e}");
    }
    println!("Local editor ready: http://{ADDRESS}");
    for incoming in listener.incoming() {
        let mut stream = incoming.map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let r = match request(&mut stream) {
            Ok(r) => r,
            Err(e) => {
                respond(
                    &mut stream,
                    400,
                    "application/json",
                    &json!({"error":e}).to_string(),
                );
                continue;
            }
        };
        if !authorized(&r, &token) {
            respond(
                &mut stream,
                403,
                "application/json",
                "{\"error\":\"Local editor request rejected\"}",
            );
            continue;
        }
        if r.method == "GET" && r.path == "/api/live" {
            app.reap();
            let s = app.status.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(subscription) = s.live.subscribe() {
                s.live.publish(&s.live_frame);
                std::thread::spawn(move || subscription.stream(stream));
            } else {
                respond(
                    &mut stream,
                    429,
                    "application/json",
                    "{\"error\":\"At most four live views can be open at once\"}",
                );
            }
            continue;
        }
        let mut exiting = false;
        match (r.method.as_str(), r.path.as_str()) {
            ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", &html),
            ("GET", "/fonts/inter-693b77d4.woff2") => respond_bytes(
                &mut stream,
                200,
                "font/woff2",
                include_bytes!("../web/fonts/InterVariable.woff2"),
            ),
            ("GET", "/fonts/LICENSE.txt") => respond(
                &mut stream,
                200,
                "text/plain; charset=utf-8",
                include_str!("../web/fonts/LICENSE.txt"),
            ),
            ("GET", "/app.js") => respond(
                &mut stream,
                200,
                "text/javascript; charset=utf-8",
                include_str!("../web/app.js"),
            ),
            ("GET", "/layout.js") => respond(
                &mut stream,
                200,
                "text/javascript; charset=utf-8",
                include_str!("../web/layout.js"),
            ),
            ("GET", "/macro-test") => respond(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                include_str!("../web/macro-test.html"),
            ),
            ("GET", "/recovery-test") => respond(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                include_str!("../web/recovery-test.html"),
            ),
            ("GET", "/mouse-test") => respond(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                include_str!("../web/mouse-test.html"),
            ),
            ("GET", "/mouse-test.js") => respond(
                &mut stream,
                200,
                "text/javascript; charset=utf-8",
                include_str!("../web/mouse-test.js"),
            ),
            ("GET", "/style.css") => respond(
                &mut stream,
                200,
                "text/css; charset=utf-8",
                include_str!("../web/style.css"),
            ),
            _ if r.path.starts_with("/api/") => match app.api(&r, &token) {
                Ok(value) => {
                    exiting = value.get("exiting").and_then(|v| v.as_bool()) == Some(true);
                    respond(&mut stream, 200, "application/json", &value.to_string());
                }
                Err(e) => respond(
                    &mut stream,
                    if e.starts_with("Conflict:") { 409 } else { 400 },
                    "application/json",
                    &json!({"error":e}).to_string(),
                ),
            },
            _ => respond(&mut stream, 404, "text/plain", "Not found"),
        }
        if exiting {
            break;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ambiguous_or_oversized_http_requests() {
        for raw in [
            "POST /api/config HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n{}".to_string(),
            "POST /api/config HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n".to_string(),
            "POST /api/config HTTP/1.1\r\nContent-Length: 150001\r\n\r\n".to_string(),
            "POST /api/config HTTP/1.1\r\nContent-Type: text/plain\r\n\r\n".to_string(),
            format!("GET / HTTP/1.1\r\nX-Large: {}\r\n\r\n", "x".repeat(8192)),
            "POST /api/config HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{".to_string(),
        ] { assert!(request(&mut std::io::Cursor::new(raw)).is_err()); }
        let raw = "POST /api/stop HTTP/1.1\r\nHost: 127.0.0.1:17832\r\nContent-Type: application/json\r\nContent-Length: 2\r\nX-Tartarus-Token: secret\r\n\r\n{}";
        let r = request(&mut std::io::Cursor::new(raw)).unwrap();
        assert!(authorized(&r, "secret"));
        assert_eq!(r.body, "{}");
    }
    #[test]
    fn local_api_rejects_foreign_hosts_origins_and_missing_tokens() {
        let mut r = Request {
            method: "POST".into(),
            path: "/api/start".into(),
            host: ADDRESS.into(),
            origin: None,
            token: Some("secret".into()),
            body: String::new(),
        };
        assert!(authorized(&r, "secret"));
        r.origin = Some("https://example.com".into());
        assert!(!authorized(&r, "secret"));
        r.origin = None;
        r.host = "attacker.example:17832".into();
        assert!(!authorized(&r, "secret"));
        r.host = ADDRESS.into();
        r.token = None;
        assert!(!authorized(&r, "secret"));
    }
    #[test]
    fn invalid_or_stale_save_preserves_configuration() {
        let dir = std::env::current_dir()
            .unwrap()
            .join("artifacts")
            .join(format!("tartarus-save-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = r#"{"schema_version":1}"#;
        fs::write(&path, original).unwrap();
        assert!(save(&path, original, "{}", "test").is_err());
        assert!(save(&path, "stale", original, "test").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let updated = r#"{"schema_version":1,"mappings":{"01":{"type":"key","key":"C"}}}"#;
        save(&path, original, updated, "test").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), updated);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
