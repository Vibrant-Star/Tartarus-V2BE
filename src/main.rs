//! Diagnostics and explicit, bounded interception feasibility trial.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(windows)]
mod cleanup;
mod config;
#[cfg(windows)]
mod diagnostics;
#[cfg(windows)]
mod interception;
mod layers;
#[cfg(windows)]
mod lighting;
#[cfg(windows)]
mod lighting_hid;
mod live;
mod macros;
mod mapping;
mod mouse;
#[cfg(windows)]
mod recovery;
#[cfg(windows)]
mod runtime;
#[cfg(windows)]
mod startup;
mod synapse;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod web;

fn main() -> std::process::ExitCode {
    #[cfg(windows)]
    let result = run();
    #[cfg(not(windows))]
    let result: Result<(), String> = Err("Windows x64 is required".into());
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    if command.as_deref() == Some(std::ffi::OsStr::new("lighting-brightness")) {
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&lighting::read_brightness()?)
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("lighting-probe")) {
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&lighting_hid::enumerate()?).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if command.is_none() || command.as_deref() == Some(std::ffi::OsStr::new("launch")) {
        let mut root = None;
        let mut background = false;
        let mut open = false;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--background") => background = true,
                Some("--open") => open = true,
                Some("--root") => {
                    root = Some(std::path::PathBuf::from(
                        args.next().ok_or("Missing --root path")?,
                    ))
                }
                _ => return Err("Unknown launch option".into()),
            }
        }
        let result = (|| {
            let root = match root {
                Some(root) => root,
                None => startup::root()?,
            };
            startup::launch(root, background, open)
        })();
        if let Err(message) = &result {
            eprintln!("Launch failed: {message}");
            startup::show_error(message);
        }
        return result;
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("disable-autostart")) {
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return startup::disable();
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("preview-synapse")) {
        let path = args.next().ok_or("Supply a Synapse 3 backup path")?;
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        use std::io::Read;
        let path = std::path::Path::new(&path);
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take((synapse::MAX_ARCHIVE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let result = synapse::preview(
            &path.file_name().unwrap_or_default().to_string_lossy(),
            &bytes,
        )?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("ui")) {
        let dll = args.next().ok_or("Supply absolute DLL path")?;
        let config = args.next().ok_or("Supply JSON configuration path")?;
        let logs = args.next().ok_or("Supply runtime log directory")?;
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return web::serve(dll.into(), config.into(), logs.into(), false);
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("check-config")) {
        let path = args.next().ok_or("Supply JSON configuration path")?;
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        let (_, bindings) = config::load(std::path::Path::new(&path))?;
        println!(
            "Configuration valid: {} explicit control mappings",
            bindings
                .layers
                .iter()
                .map(|layer| layer.len())
                .sum::<usize>()
        );
        return Ok(());
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("stop")) {
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return runtime::stop();
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("run")) {
        let dll = args.next().ok_or("Supply DLL path")?;
        let config = args.next().ok_or("Supply JSON configuration path")?;
        let log = args.next().ok_or("Supply new runtime log path")?;
        let seconds = match args.next() {
            Some(value) => Some(
                value
                    .to_str()
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|n| (1..=300).contains(n))
                    .ok_or("Duration must be 1–300 seconds")?,
            ),
            None => None,
        };
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return runtime::run(
            std::path::Path::new(&dll),
            std::path::Path::new(&config),
            std::path::Path::new(&log),
            seconds,
        );
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("trial")) {
        let dll = args
            .next()
            .ok_or("Supply an absolute Interception DLL path")?;
        let seconds = args
            .next()
            .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
            .ok_or("Supply trial duration")?;
        let output = args.next().ok_or("Supply a new trial log path")?;
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return interception::trial(
            std::path::Path::new(&dll),
            seconds,
            std::path::Path::new(&output),
        );
    }
    if matches!(
        command.as_deref().and_then(|s| s.to_str()),
        Some("capture" | "verify")
    ) {
        let seconds: u32 = args
            .next()
            .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
            .filter(|n| (1..=300).contains(n))
            .ok_or("Capture duration must be 1–300 seconds")?;
        let output = args.next().ok_or("Supply a new output trace path")?;
        if args.next().is_some() {
            return Err("Unexpected extra arguments".into());
        }
        return if command.as_deref() == Some(std::ffi::OsStr::new("verify")) {
            diagnostics::verify(seconds, std::path::Path::new(&output))
        } else {
            diagnostics::capture(seconds, std::path::Path::new(&output))
        };
    }
    if command.as_deref() != Some(std::ffi::OsStr::new("probe")) {
        return Err("Usage: tartarus-v2be probe <absolute-DLL-path>\n       tartarus-v2be capture|verify <seconds:1–300> <new-trace-path>\n       tartarus-v2be trial <absolute-DLL-path> <seconds:1–60> <new-log-path>\n       tartarus-v2be check-config <JSON-path>\n       tartarus-v2be run <absolute-DLL-path> <JSON-path> <new-log-path> [seconds:1–300]\n       tartarus-v2be stop\n       tartarus-v2be ui <absolute-DLL-path> <JSON-path> <log-directory>\nOnly explicit trial/run commands or Start remapping in the UI enable interception.".into());
    }
    let path = args.next().ok_or("Supply the absolute DLL path")?;
    if args.next().is_some() {
        return Err("Unexpected extra arguments".into());
    }
    interception::probe(std::path::Path::new(&path))
}
