//! V2-only hardware lighting protocol. Independently encoded from protocol facts.
use crate::lighting_hid;
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Off,
    Static,
    Breathing,
    Spectrum,
    Wave,
    Reactive,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub managed: bool,
    pub effect: Effect,
    pub color: [u8; 3],
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            managed: false,
            effect: Effect::Spectrum,
            color: [0, 255, 80],
            brightness: 75,
            speed: 2,
            direction: 1,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.brightness > 100
            || !(1..=4).contains(&self.speed)
            || !(1..=2).contains(&self.direction)
        {
            return Err("Lighting requires schema 1, brightness 0–100, reactive speed 1–4 and wave direction 1 or 2".into());
        }
        Ok(())
    }
}
pub fn report(command: u8, args: &[u8]) -> [u8; 91] {
    assert!(args.len() <= 80);
    let mut p = [0u8; 91];
    p[2] = 0x1f;
    p[6] = args.len() as u8;
    p[7] = 0x0f;
    p[8] = command;
    p[9..9 + args.len()].copy_from_slice(args);
    p[89] = p[3..89].iter().fold(0, |a, b| a ^ b);
    p
}
fn response(request: &[u8; 91], reply: &[u8; 91]) -> Result<bool, String> {
    if reply[0] != 0 || reply[2] != request[2] || reply[7..9] != request[7..9] {
        return Ok(false);
    }
    if reply[6] > 80
        || reply[3..6] != [0, 0, 0]
        || reply[90] != 0
        || reply[89] != reply[3..89].iter().fold(0, |a, b| a ^ b)
    {
        return Err("Lighting response framing/checksum failed".into());
    }
    match reply[1] {
        0 | 1 => Ok(false),
        2 => Ok(true),
        3 => Err("Device rejected lighting command".into()),
        4 => Err("Device lighting command timed out".into()),
        5 => Err("Device reports lighting command unsupported".into()),
        s => Err(format!("Unknown lighting response status {s}")),
    }
}
fn query(handle: &lighting_hid::Handle, mut request: [u8; 91]) -> Result<[u8; 91], String> {
    handle.set_feature(&mut request)?;
    for _ in 0..15 {
        thread::sleep(Duration::from_millis(5));
        let reply = handle.get_feature()?;
        if response(&request, &reply)? {
            return Ok(reply);
        }
    }
    Err("No matching lighting response; another RGB manager may own the device".into())
}
pub fn read_brightness() -> Result<serde_json::Value, String> {
    let _guard = Guard::lock()?;
    let handle = lighting_hid::control()?;
    let reply = query(&handle, report(0x84, &[0, 5, 0]))?;
    if reply[6] < 3 || reply[9..11] != [0, 5] {
        return Err("Unexpected lighting brightness payload".into());
    }
    Ok(
        serde_json::json!({"raw":reply[11],"percent":(u32::from(reply[11])*100+127)/255,"response":reply.to_vec()}),
    )
}
fn effect_report(s: &Settings) -> [u8; 91] {
    let mut args = vec![
        0,
        5,
        match s.effect {
            Effect::Off => 0,
            Effect::Static => 1,
            Effect::Breathing => 2,
            Effect::Spectrum => 3,
            Effect::Wave => 4,
            Effect::Reactive => 5,
        },
        0,
        0,
        0,
    ];
    match s.effect {
        Effect::Static | Effect::Breathing | Effect::Reactive => {
            args[5] = 1;
            if s.effect == Effect::Breathing {
                args[3] = 1
            }
            if s.effect == Effect::Reactive {
                args[4] = s.speed
            }
            args.extend(s.color);
        }
        Effect::Wave => {
            args[3] = s.direction;
            args[4] = 0x28
        }
        _ => {}
    }
    report(2, &args)
}
pub fn apply(s: &Settings) -> Result<(), String> {
    s.validate()?;
    if !s.managed {
        return Ok(());
    }
    let _guard = Guard::lock()?;
    let handle = lighting_hid::control()?;
    let brightness = ((u16::from(s.brightness) * 255 + 50) / 100) as u8;
    query(&handle, report(4, &[0, 5, brightness]))?;
    query(&handle, effect_report(s))?;
    let reply = query(&handle, report(0x84, &[0, 5, 0]))?;
    if reply[6] < 3 || reply[9..11] != [0, 5] || reply[11] != brightness {
        return Err("Lighting brightness readback did not match".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unmanaged_worker_persists_and_rejects_stale_edits_without_device_commands() {
        let dir = std::env::current_dir()
            .unwrap()
            .join("artifacts")
            .join(format!("lighting-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let controller = Controller::new(&dir).unwrap();
        assert!(
            !controller.view().unwrap()["settings"]["managed"]
                .as_bool()
                .unwrap()
        );
        controller
            .save(Update {
                expected: None,
                settings: Settings::default(),
            })
            .unwrap();
        assert!(
            controller
                .save(Update {
                    expected: None,
                    settings: Settings::default()
                })
                .is_err()
        );
        drop(controller);
        std::fs::remove_file(dir.join("lighting.json")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn command_bytes_and_checksum_are_v2_specific() {
        let s = Settings {
            managed: true,
            effect: Effect::Static,
            color: [0x12, 0x34, 0x56],
            ..Settings::default()
        };
        let p = effect_report(&s);
        assert_eq!(
            &p[..18],
            &[
                0, 0, 0x1f, 0, 0, 0, 9, 0x0f, 2, 0, 5, 1, 0, 0, 1, 0x12, 0x34, 0x56
            ]
        );
        assert_eq!(p[89], p[3..89].iter().fold(0, |a, b| a ^ b));
        let mut reply = p;
        reply[1] = 2;
        assert_eq!(response(&p, &reply), Ok(true));
        reply[2] = 0x3f;
        assert_eq!(response(&p, &reply), Ok(false));
        reply = p;
        reply[1] = 5;
        assert!(response(&p, &reply).is_err());
        reply = p;
        reply[1] = 2;
        reply[15] ^= 1;
        assert!(response(&p, &reply).is_err());
        assert_eq!(report(0x84, &[0, 5, 0])[6], 3);
    }
    #[test]
    fn settings_reject_invalid_values_and_encode_hardware_effects() {
        let mut s = Settings::default();
        for (effect, id, len) in [
            (Effect::Off, 0, 6),
            (Effect::Static, 1, 9),
            (Effect::Breathing, 2, 9),
            (Effect::Spectrum, 3, 6),
            (Effect::Wave, 4, 6),
            (Effect::Reactive, 5, 9),
        ] {
            s.effect = effect;
            let p = effect_report(&s);
            assert_eq!((p[11], p[6]), (id, len));
        }
        s.brightness = 101;
        assert!(s.validate().is_err());
        s.brightness = 50;
        s.direction = 0;
        assert!(s.validate().is_err());
    }
}

// Dedicated hardware-effect worker: latest desired state, no animation timer.
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(a: *const std::ffi::c_void, owner: i32, name: *const u16) -> isize;
    fn ReleaseMutex(handle: isize) -> i32;
    fn CreateEventW(
        a: *const std::ffi::c_void,
        manual: i32,
        initial: i32,
        name: *const u16,
    ) -> isize;
    fn SetEvent(event: isize) -> i32;
    fn WaitForSingleObject(event: isize, ms: u32) -> u32;
    fn CloseHandle(event: isize) -> i32;
}
struct Guard(isize);
impl Guard {
    fn lock() -> Result<Self, String> {
        let name: Vec<_> = "Local\\TartarusV2BE.Lighting.1532.022B"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle == 0 {
            return Err("Cannot create lighting coordination mutex".into());
        }
        if !matches!(unsafe { WaitForSingleObject(handle, 1000) }, 0 | 0x80) {
            unsafe {
                CloseHandle(handle);
            }
            return Err("Another Tartarus V2BE instance is using lighting".into());
        }
        Ok(Self(handle))
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}
struct State {
    desired: Settings,
    revision: u64,
    applied: bool,
    phase: String,
    error: Option<String>,
}
pub struct Controller {
    shared: Arc<Mutex<State>>,
    wake: isize,
    stop: isize,
    thread: Option<thread::JoinHandle<()>>,
    path: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    expected: Option<String>,
    settings: Settings,
}
fn saved(path: &Path) -> Result<Option<String>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut text = String::new();
    file.take(4097)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > 4096 {
        return Err("Lighting settings exceed 4 KiB".into());
    }
    Ok(Some(text))
}
impl Controller {
    pub fn new(data: &Path) -> Result<Self, String> {
        let path = data.join("lighting.json");
        let loaded = saved(&path).and_then(|text| match text {
            Some(text) => serde_json::from_str::<Settings>(&text)
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    s.validate()?;
                    Ok(s)
                }),
            None => Ok(Settings::default()),
        });
        let error = loaded.as_ref().err().cloned();
        let shared = Arc::new(Mutex::new(State {
            desired: loaded.unwrap_or_default(),
            revision: 1,
            applied: false,
            phase: "Lighting unchanged".into(),
            error,
        }));
        let stop = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        if stop == 0 {
            return Err("Cannot create lighting stop event".into());
        }
        let worker_shared = shared.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            let monitor = match crate::recovery::Monitor::lighting() {
                Ok(m) => m,
                Err(e) => {
                    let _ = sender.send(Err(e));
                    return;
                }
            };
            if sender.send(Ok(monitor.wake_handle())).is_err() {
                return;
            }
            let mut generation = monitor.generation();
            let mut applied_revision = 0;
            let mut failures: u32 = 0;
            loop {
                if unsafe { WaitForSingleObject(stop, 0) } == 0 {
                    break;
                }
                let now = monitor.generation();
                if now != generation {
                    generation = now;
                    applied_revision = 0;
                }
                let (desired, revision) = {
                    let s = worker_shared.lock().unwrap_or_else(|e| e.into_inner());
                    (s.desired.clone(), s.revision)
                };
                let mut delay = u32::MAX;
                if !desired.managed || monitor.suspended() {
                    let mut s = worker_shared.lock().unwrap_or_else(|e| e.into_inner());
                    s.applied = false;
                    s.phase = if monitor.suspended() {
                        "Lighting waiting for wake"
                    } else {
                        "Lighting unchanged"
                    }
                    .into();
                } else if applied_revision != revision {
                    let outcome = apply(&desired);
                    let mut s = worker_shared.lock().unwrap_or_else(|e| e.into_inner());
                    if s.revision == revision
                        && generation == monitor.generation()
                        && !monitor.suspended()
                    {
                        match outcome {
                            Ok(()) => {
                                applied_revision = revision;
                                failures = 0;
                                s.applied = true;
                                s.error = None;
                                s.phase = "Hardware lighting applied".into();
                            }
                            Err(e) => {
                                failures = failures.saturating_add(1);
                                s.applied = false;
                                s.error = Some(e);
                                s.phase = "Lighting unavailable; waiting to retry".into();
                                delay = match failures {
                                    1 => 1000,
                                    2 => 2000,
                                    3 => 5000,
                                    _ => 30000,
                                };
                            }
                        }
                    } else {
                        delay = 0;
                    }
                }
                if let Err(e) = monitor.wait(stop, delay) {
                    let mut s = worker_shared.lock().unwrap_or_else(|e| e.into_inner());
                    s.applied = false;
                    s.error = Some(e);
                    s.phase = "Lighting worker stopped".into();
                    break;
                }
            }
        });
        let wake = match receiver.recv() {
            Ok(Ok(wake)) => wake,
            result => {
                let _ = thread.join();
                unsafe {
                    CloseHandle(stop);
                }
                return Err(format!("Cannot start lighting monitor: {result:?}"));
            }
        };
        Ok(Self {
            shared,
            wake,
            stop,
            thread: Some(thread),
            path,
        })
    }
    pub fn observation(&self) -> serde_json::Value {
        let s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        serde_json::json!({"applied":s.applied,"status":s.phase,"error":s.error,"revision":s.revision})
    }
    pub fn view(&self) -> Result<serde_json::Value, String> {
        let expected = saved(&self.path)?;
        let s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        Ok(
            serde_json::json!({"settings":s.desired,"expected":expected,"revision":s.revision,"applied":s.applied,"status":s.phase,"error":s.error}),
        )
    }
    pub fn save(&self, u: Update) -> Result<serde_json::Value, String> {
        u.settings.validate()?;
        if saved(&self.path)? != u.expected {
            return Err("Conflict: lighting settings changed elsewhere; reload them first".into());
        }
        crate::startup::replace(
            &self.path,
            &serde_json::to_string_pretty(&u.settings).map_err(|e| e.to_string())?,
        )?;
        {
            let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            s.desired = u.settings;
            s.revision = s.revision.saturating_add(1);
            s.applied = false;
            s.phase = "Lighting settings saved; applying".into();
            s.error = None;
        }
        unsafe {
            SetEvent(self.wake);
        }
        self.view()
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        unsafe {
            SetEvent(self.stop);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        unsafe {
            CloseHandle(self.stop);
        }
    }
}
