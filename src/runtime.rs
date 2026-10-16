use crate::{
    config,
    interception::{DeviceBackend, DeviceEvent},
    layers::LayerEngine,
    mapping::{Key, Output},
};
use std::{
    ffi::c_void,
    io::Write,
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::atomic::{AtomicIsize, Ordering},
    time::{Duration, Instant},
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateEventW(
        attributes: *const c_void,
        manual: i32,
        initial: i32,
        name: *const u16,
    ) -> isize;
    fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> isize;
    fn SetEvent(handle: isize) -> i32;
    fn CloseHandle(handle: isize) -> i32;
    fn SetLastError(code: u32);
    fn WaitForSingleObject(handle: isize, milliseconds: u32) -> u32;
    fn SetConsoleCtrlHandler(handler: unsafe extern "system" fn(u32) -> i32, add: i32) -> i32;
    fn FindFirstChangeNotificationW(path: *const u16, subtree: i32, filter: u32) -> isize;
    fn FindNextChangeNotification(handle: isize) -> i32;
    fn FindCloseChangeNotification(handle: isize) -> i32;
}
static STOP: AtomicIsize = AtomicIsize::new(0);
#[derive(Default, serde::Serialize)]
pub struct Status {
    pub running: bool,
    pub recovering: Option<String>,
    pub applied_text: String,
    pub layer: usize,
    #[serde(skip)]
    pub live: std::sync::Arc<crate::live::Hub>,
    #[serde(skip)]
    pub live_frame: crate::live::Frame,
}
pub type SharedStatus = std::sync::Arc<std::sync::Mutex<Status>>;
fn event_name() -> Vec<u16> {
    "Local\\TartarusV2BE.KeyboardRuntime.Stop\0"
        .encode_utf16()
        .collect()
}
unsafe extern "system" fn control(code: u32) -> i32 {
    if matches!(code, 0 | 1 | 2 | 5 | 6) {
        let handle = STOP.load(Ordering::SeqCst);
        if handle != 0 {
            unsafe {
                SetEvent(handle);
            }
            return 1;
        }
    }
    0
}
struct StopEvent(isize);
impl Drop for StopEvent {
    fn drop(&mut self) {
        STOP.store(0, Ordering::SeqCst);
        unsafe {
            SetConsoleCtrlHandler(control, 0);
            CloseHandle(self.0);
        }
    }
}
struct Watch(isize);
impl Drop for Watch {
    fn drop(&mut self) {
        unsafe {
            FindCloseChangeNotification(self.0);
        }
    }
}

pub fn stop() -> Result<(), String> {
    let handle = unsafe { OpenEventW(2, 0, event_name().as_ptr()) };
    if handle == 0 {
        return Err("No accessible keyboard runtime is running".into());
    }
    let result = unsafe { SetEvent(handle) };
    let error = std::io::Error::last_os_error();
    unsafe {
        CloseHandle(handle);
    }
    if result == 0 {
        Err(error.to_string())
    } else {
        Ok(())
    }
}

struct Running {
    // Actual successfully sent downs, not merely intended engine state.
    sent: Vec<Key>,
    backend: DeviceBackend,
}
impl Running {
    fn cleanup(&mut self, device_lost: bool) -> Result<(), String> {
        while let Some(key) = self.sent.last().copied() {
            if device_lost || self.backend.emit(Output { key, up: true }).is_err() {
                crate::cleanup::release_owned(key)?;
            }
            self.sent.pop();
        }
        Ok(())
    }
    fn deliver(&mut self, outputs: Vec<Output>) -> Result<(), String> {
        for output in outputs {
            self.backend.emit(output)?;
            if output.up {
                self.sent.retain(|k| *k != output.key);
            } else if !crate::mouse::is_wheel(output.key) && !self.sent.contains(&output.key) {
                self.sent.push(output.key);
            }
        }
        Ok(())
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup(false) {
            eprintln!("Output cleanup incomplete: {error}");
        }
    }
}

pub fn run(dll: &Path, path: &Path, output: &Path, seconds: Option<u32>) -> Result<(), String> {
    run_monitored(dll, path, output, seconds, &SharedStatus::default())
}
pub fn run_monitored(
    dll: &Path,
    path: &Path,
    output: &Path,
    seconds: Option<u32>,
    status: &SharedStatus,
) -> Result<(), String> {
    let result = run_inner(dll, path, output, seconds, status);
    {
        let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
        s.running = false;
        s.recovering = None;
        s.live_frame = crate::live::Frame::default();
        s.live.publish(&s.live_frame);
    }
    result
}
fn run_inner(
    dll: &Path,
    path: &Path,
    output: &Path,
    seconds: Option<u32>,
    status: &SharedStatus,
) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let event = unsafe {
        SetLastError(0);
        CreateEventW(std::ptr::null(), 1, 0, event_name().as_ptr())
    };
    let error = std::io::Error::last_os_error();
    if event == 0 {
        return Err(error.to_string());
    }
    if error.raw_os_error() == Some(183) {
        unsafe {
            CloseHandle(event);
        }
        return Err("A keyboard runtime already exists".into());
    }
    let stop = StopEvent(event);
    STOP.store(event, Ordering::SeqCst);
    if unsafe { SetConsoleCtrlHandler(control, 1) } == 0 {
        return Err("Cannot register shutdown control handler".into());
    }
    let directory: Vec<u16> = path
        .parent()
        .unwrap()
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let watch_handle = unsafe { FindFirstChangeNotificationW(directory.as_ptr(), 0, 1 | 8 | 16) };
    if watch_handle == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let watch = Watch(watch_handle);
    let monitor = crate::recovery::Monitor::new()?;
    let (mut current_text, mut profile) = config::load(&path)?;
    let start = Instant::now();
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_micros();
    let mut revision = 1;
    let mut inputs = 0;
    let mut rejected: Option<String> = None;
    let mut attempts: u32 = 0;
    let mut recovery_reason: Option<String> = None;
    loop {
        if unsafe { WaitForSingleObject(stop.0, 0) } == 0
            || seconds.is_some_and(|s| start.elapsed() >= Duration::from_secs(s.into()))
        {
            break;
        }
        let generation = monitor.generation();
        let connected = if monitor.suspended() {
            Err("Windows suspended; waiting for resume".into())
        } else {
            DeviceBackend::connect(dll)
        };
        let backend = match connected {
            Ok(backend) => backend,
            Err(reason) => {
                if recovery_reason.as_ref() != Some(&reason) {
                    writeln!(log, "# recovery-wait reason={reason:?}")
                        .map_err(|e| e.to_string())?;
                    recovery_reason = Some(reason.clone());
                }
                {
                    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
                    s.running = false;
                    s.recovering = Some(reason);
                    s.layer = 0;
                    s.live_frame = crate::live::Frame::default();
                    s.live.publish(&s.live_frame);
                }
                monitor.acknowledge();
                let mut delay = if monitor.suspended() {
                    u32::MAX
                } else if recovery_reason
                    .as_deref()
                    .is_some_and(|s| s.starts_with("Release all"))
                {
                    250
                } else {
                    crate::recovery::retry_delay(attempts)
                };
                if let Some(seconds) = seconds {
                    delay = delay.min(
                        Duration::from_secs(seconds.into())
                            .saturating_sub(start.elapsed())
                            .as_millis()
                            .min(u128::from(u32::MAX)) as u32,
                    );
                }
                monitor.wait(stop.0, delay)?;
                attempts = attempts.saturating_add(1);
                continue;
            }
        };
        if generation != monitor.generation() || monitor.suspended() {
            drop(backend);
            continue;
        }
        // Fresh context and engine: stale slots, partial wheel deltas, held states
        // and old layers never cross a reconnect boundary.
        if let Ok((text, next)) = config::load(&path)
            && text != current_text
        {
            current_text = text;
            profile = next;
            revision += 1;
        }
        let mut physical = crate::live::Physical::new(&profile, &current_text);
        let mut live_mouse = crate::mouse::MouseMapper::default();
        let live_mouse_bindings = crate::live::Physical::mouse_bindings();
        let mut mouse_mapper = crate::mouse::MouseMapper::default();
        let mut engine = LayerEngine::new(profile.clone());
        let mut running = Running {
            sent: vec![],
            backend,
        };
        attempts = 0;
        recovery_reason = None;
        writeln!(log,"# runtime active keyboard-and-mouse live_reload=true bounded_seconds={seconds:?} filter_query_unreliable=true start_epoch_us={epoch}").map_err(|e| e.to_string())?;
        {
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            s.running = true;
            s.recovering = None;
            s.applied_text = current_text.clone();
            s.layer = 0;
            s.live_frame = physical.frame.clone();
            s.live.publish(&s.live_frame);
        }
        let session_result = (|| -> Result<(), String> {
            loop {
                if unsafe { WaitForSingleObject(stop.0, 0) } == 0
                    || seconds.is_some_and(|s| start.elapsed() >= Duration::from_secs(s.into()))
                {
                    break;
                }
                if generation != monitor.generation() || monitor.suspended() {
                    return Err("Device or power state changed; rebuilding capture".into());
                }
                match unsafe { WaitForSingleObject(watch.0, 0) } {
                    0 => {
                        if unsafe { FindNextChangeNotification(watch.0) } == 0 {
                            return Err("Config watcher failed".into());
                        }
                        match config::load(&path) {
                            Ok((text, bindings)) if text != current_text => {
                                let mut next = crate::live::Physical::new(&bindings, &text);
                                next.frame.held = physical.frame.held.clone();
                                next.frame.presses = physical.frame.presses.clone();
                                physical = next;
                                mouse_mapper.reset_wheel();
                                profile = bindings.clone();
                                running.deliver(engine.replace(bindings))?;
                                current_text = text;
                                {
                                    let mut observed =
                                        status.lock().unwrap_or_else(|e| e.into_inner());
                                    observed.applied_text = current_text.clone();
                                    observed.layer = 0;
                                    observed.live_frame = physical.frame.clone();
                                    observed.live.publish(&observed.live_frame);
                                }
                                revision += 1;
                                rejected = None;
                                writeln!(
                                    log,
                                    "# config-applied revision={revision} owned={} elapsed_us={}",
                                    engine.owned_count(),
                                    start.elapsed().as_micros()
                                )
                                .map_err(|e| e.to_string())?;
                            }
                            Ok(_) => {}
                            Err(error) if rejected.as_ref() != Some(&error) => {
                                writeln!(
                                    log,
                                    "# config-rejected last-valid-kept=true reason={error:?}"
                                )
                                .map_err(|e| e.to_string())?;
                                rejected = Some(error);
                            }
                            Err(_) => {}
                        }
                    }
                    258 => {}
                    _ => return Err("Config change wait failed".into()),
                }
                // The public Interception API has no combined wait for a config/stop event.
                // A blocking 250-ms timeout bounds control latency; there is no file polling.
                running.deliver(engine.tick(Instant::now()))?;
                if let Some(event) = running.backend.next(engine.wait_ms())? {
                    if generation != monitor.generation() || monitor.suspended() {
                        return Err("Device or power state changed during input wait".into());
                    }
                    inputs += 1;
                    let previous_layer = engine.active();
                    match event {
                        DeviceEvent::Keyboard(key, up, information, state) => {
                            if key == crate::layers::THUMB && state & !7 == 0 {
                                let kind = if up {
                                    "release"
                                } else if engine.thumb_held() {
                                    "repeat"
                                } else {
                                    "press"
                                };
                                writeln!(
                                    log,
                                    "# thumb-input kind={kind} layer={} elapsed_us={}",
                                    engine.active(),
                                    start.elapsed().as_micros()
                                )
                                .map_err(|e| e.to_string())?;
                            }
                            if state & !7 != 0 {
                                running.backend.forward_special(key, state, information)?;
                            } else {
                                physical.event(key, up);
                                running.deliver(engine.event(key, up))?;
                            }
                        }
                        DeviceEvent::Mouse(packet) => {
                            for (key, up) in live_mouse.split(packet, &live_mouse_bindings).1 {
                                physical.event(key, up);
                            }
                            let (residual, changes) = mouse_mapper.split(packet, engine.bindings());
                            if residual.has_event() {
                                running.backend.forward_mouse(residual)?;
                            }
                            for (control, up) in changes {
                                if matches!(
                                    control,
                                    crate::mouse::WHEEL_UP | crate::mouse::WHEEL_DOWN
                                ) {
                                    if !up {
                                        running.deliver(engine.pulse(control))?;
                                    }
                                } else {
                                    running.deliver(engine.event(control, up))?;
                                }
                            }
                        }
                    }
                    if engine.active() != previous_layer {
                        status.lock().unwrap_or_else(|e| e.into_inner()).layer = engine.active();
                        mouse_mapper.reset_wheel();
                        writeln!(
                            log,
                            "# layer-changed from={previous_layer} to={} elapsed_us={} owned={}",
                            engine.active(),
                            start.elapsed().as_micros(),
                            engine.owned_count()
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    physical.frame.layer = engine.active();
                    {
                        let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
                        s.live_frame = physical.frame.clone();
                        s.live.publish(&s.live_frame);
                    }
                }
            }
            Ok(())
        })();
        // Discard intended state; cleanup uses only actual delivered holds.
        let _ = engine.release_all();
        let cleanup = running.cleanup(session_result.is_err());
        drop(running);
        monitor.acknowledge();
        cleanup?;
        {
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            s.running = false;
            s.recovering = session_result.as_ref().err().cloned();
            s.live_frame = crate::live::Frame::default();
            s.live.publish(&s.live_frame);
        }
        match session_result {
            Ok(()) => break,
            Err(reason) => {
                writeln!(log, "# capture-reset owned=0 reason={reason:?}")
                    .map_err(|e| e.to_string())?;
                monitor.wait(stop.0, 1000)?;
            }
        }
    }
    writeln!(
        log,
        "# runtime stopped inputs={inputs} revisions={revision} owned=0 filter_clear_requested=true"
    )
    .map_err(|e| e.to_string())?;
    log.flush().map_err(|e| e.to_string())?;
    Ok(())
}
