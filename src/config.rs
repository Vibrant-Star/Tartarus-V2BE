use crate::mapping::{Bindings, Key};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    #[serde(default, deserialize_with = "unique_mappings")]
    mappings: BTreeMap<String, Action>,
    #[serde(default)]
    layers: Vec<Layer>,
    #[serde(default)]
    thumb: Option<Thumb>,
    #[serde(default)]
    source_import: Option<crate::synapse::SourceImport>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    name: String,
    #[serde(default, deserialize_with = "unique_mappings")]
    mappings: BTreeMap<String, Action>,
}
#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum Thumb {
    Modifier {
        #[serde(default = "default_modifier")]
        key: String,
    },
    Momentary {
        layer: usize,
    },
    Cycle {
        layers: Vec<usize>,
    },
}
fn default_modifier() -> String {
    "LeftAlt".into()
}
fn unique_mappings<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Action>, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, Action>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map of unique control names")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, Action>()? {
                if result.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate control {key:?}"
                    )));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Unique)
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Macro { steps: Vec<MacroStep> },
    Key { key: String },
    MouseButton { button: String },
    MouseWheel { direction: String },
    Chord { keys: Vec<String> },
    Disabled,
    Passthrough,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum MacroStep {
    Tap {
        keys: Vec<String>,
        #[serde(default = "tap_duration")]
        hold_ms: u64,
    },
    MouseButton {
        button: String,
        #[serde(default = "tap_duration")]
        hold_ms: u64,
    },
    MouseWheel {
        direction: String,
    },
    Delay {
        ms: u64,
    },
}
fn tap_duration() -> u64 {
    40
}
fn compile_macro(steps: Vec<MacroStep>) -> Result<Vec<crate::macros::Step>, String> {
    if steps.is_empty() || steps.len() > 128 {
        return Err("Macros require 1–128 steps".into());
    }
    let mut result = Vec::new();
    let mut duration = 0u64;
    for step in steps {
        let (keys, ms) = match step {
            MacroStep::Delay { ms } => {
                if !(1..=10000).contains(&ms) {
                    return Err("Macro pauses require 1–10000 ms".into());
                }
                duration += ms;
                result.push(crate::macros::Step::Delay(ms));
                continue;
            }
            MacroStep::Tap { keys, hold_ms } => {
                if keys.is_empty() || keys.len() > 8 {
                    return Err("Macro taps require 1–8 distinct keys".into());
                }
                let keys = keys
                    .iter()
                    .map(|k| output(k))
                    .collect::<Result<Vec<_>, _>>()?;
                if keys.iter().collect::<std::collections::BTreeSet<_>>().len() != keys.len() {
                    return Err("Duplicate macro tap key".into());
                }
                (keys, hold_ms)
            }
            MacroStep::MouseButton { button, hold_ms } => {
                (vec![crate::mouse::button(&button)?], hold_ms)
            }
            MacroStep::MouseWheel { direction } => (vec![crate::mouse::wheel(&direction)?], 1),
        };
        if !(1..=10000).contains(&ms) {
            return Err("Macro holds require 1–10000 ms".into());
        }
        duration += ms;
        result.push(crate::macros::Step::Tap { keys, hold_ms: ms });
    }
    if duration > 60000 {
        return Err("Macro duration cannot exceed 60 seconds".into());
    }
    Ok(result)
}

pub fn control(name: &str) -> Option<Key> {
    let scans = [
        2, 3, 4, 5, 6, 0xf, 0x10, 0x11, 0x12, 0x13, 0x3a, 0x1e, 0x1f, 0x20, 0x21, 0x2a, 0x2c, 0x2d,
        0x2e, 0x39,
    ];
    if let Ok(number) = name.parse::<usize>()
        && (1..=20).contains(&number)
        && format!("{number:02}") == name
    {
        return Some(Key {
            scan: scans[number - 1],
            extended: 0,
        });
    }
    let (scan, extended) = match name {
        "wheel_up" => return Some(crate::mouse::WHEEL_UP),
        "wheel_down" => return Some(crate::mouse::WHEEL_DOWN),
        "wheel_click" => return Some(crate::mouse::MIDDLE),
        "thumb" => (0x38, 0),
        "dpad_up" => (0x48, 2),
        "dpad_down" => (0x50, 2),
        "dpad_left" => (0x4b, 2),
        "dpad_right" => (0x4d, 2),
        _ => return None,
    };
    Some(Key { scan, extended })
}

pub fn output(name: &str) -> Result<Key, String> {
    if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() {
        return Ok(Key {
            scan: match name.as_bytes()[0] {
                b'0' => 0x0b,
                d => u16::from(d - b'1') + 2,
            },
            extended: 0,
        });
    }
    let letters = [
        0x1e, 0x30, 0x2e, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18,
        0x19, 0x10, 0x13, 0x1f, 0x14, 0x16, 0x2f, 0x11, 0x2d, 0x15, 0x2c,
    ];
    if name.len() == 1 && name.as_bytes()[0].is_ascii_uppercase() {
        return Ok(Key {
            scan: letters[(name.as_bytes()[0] - b'A') as usize],
            extended: 0,
        });
    }
    let (scan, extended) = match name {
        "LeftCtrl" => (0x1d, 0),
        "RightCtrl" => (0x1d, 2),
        "LeftAlt" => (0x38, 0),
        "RightAlt" => (0x38, 2),
        "LeftShift" => (0x2a, 0),
        "RightShift" => (0x36, 0),
        "LeftWin" => (0x5b, 2),
        "RightWin" => (0x5c, 2),
        "Space" => (0x39, 0),
        "Tab" => (0xf, 0),
        "Enter" => (0x1c, 0),
        "Escape" => (1, 0),
        "Backspace" => (0xe, 0),
        "Up" => (0x48, 2),
        "Down" => (0x50, 2),
        "Left" => (0x4b, 2),
        "Right" => (0x4d, 2),
        "Home" => (0x47, 2),
        "End" => (0x4f, 2),
        "PageUp" => (0x49, 2),
        "PageDown" => (0x51, 2),
        "Insert" => (0x52, 2),
        "Delete" => (0x53, 2),
        "Minus" => (0x0c, 0),
        "VolumeMute" => (0x20, 2),
        "VolumeDown" => (0x2e, 2),
        "VolumeUp" => (0x30, 2),
        "MediaPlayPause" => (0x22, 2),
        "MediaStop" => (0x24, 2),
        "MediaPreviousTrack" => (0x10, 2),
        "MediaNextTrack" => (0x19, 2),
        _ => {
            if let Some(raw) = name.strip_prefix("scan:") {
                let (raw, extended) = raw.strip_prefix("E0:").map(|s| (s, 2)).unwrap_or((raw, 0));
                if let Ok(scan) = u16::from_str_radix(raw, 16)
                    && (1..=0x7f).contains(&scan)
                {
                    return Ok(Key { scan, extended });
                }
            }
            return Err(format!("Unsupported key name {name:?}"));
        }
    };
    Ok(Key { scan, extended })
}

fn compile_mappings(
    mappings: BTreeMap<String, Action>,
) -> Result<(Bindings, crate::macros::Bindings), String> {
    let mut bindings = Bindings::new();
    let mut macros = crate::macros::Bindings::new();
    for (name, action) in mappings {
        let input = control(&name).ok_or_else(|| format!("Unknown/unverified control {name:?}"))?;
        let keys = match action {
            Action::Macro { steps } => {
                macros.insert(input, compile_macro(steps)?);
                vec![]
            }
            Action::Passthrough => continue,
            Action::Disabled => vec![],
            Action::Key { key } => vec![output(&key)?],
            Action::MouseButton { button } => vec![crate::mouse::button(&button)?],
            Action::MouseWheel { direction } => vec![crate::mouse::wheel(&direction)?],
            Action::Chord { keys } => {
                if keys.is_empty() || keys.len() > 8 {
                    return Err("Chords require 1–8 distinct keys".into());
                }
                let parsed: Vec<Key> = keys
                    .iter()
                    .map(|key| output(key))
                    .collect::<Result<_, _>>()?;
                if parsed
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != parsed.len()
                {
                    return Err("Duplicate chord key".into());
                }
                parsed
            }
        };
        bindings.insert(input, keys);
    }
    Ok((bindings, macros))
}

pub fn parse(text: &str) -> Result<crate::layers::Profile, String> {
    if text.len() > 65536 {
        return Err("Configuration exceeds 64 KiB".into());
    }
    let config: Config = serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(text))
        .map_err(|e| e.to_string())?;
    if config.schema_version != 1 {
        return Err("Expected schema_version 1".into());
    }
    if let Some(source) = &config.source_import {
        source.validate()?;
    }
    if config.layers.len() > 2 {
        return Err("At most three total layers are supported".into());
    }
    if config.thumb.is_some()
        && (config.mappings.contains_key("thumb")
            || config
                .layers
                .iter()
                .any(|l| l.mappings.contains_key("thumb")))
    {
        return Err("Configure thumb mode or thumb mappings, not both".into());
    }
    let (base, base_macros) = compile_mappings(config.mappings)?;
    let mut layers = vec![base];
    let mut macros = vec![base_macros];
    let mut names: std::collections::BTreeSet<String> = ["base".into()].into();
    for layer in config.layers {
        if layer.name.trim().is_empty()
            || layer.name.len() > 64
            || !names.insert(layer.name.to_lowercase())
        {
            return Err("Layer names must be unique, nonempty and at most 64 bytes".into());
        }
        let (bindings, sequences) = compile_mappings(layer.mappings)?;
        layers.push(bindings);
        macros.push(sequences);
    }
    // Keep native Tartarus middle-click on the ownership path in every layer
    // whenever any layer can emit middle-click. Explicit remaps still win.
    let middle = crate::mouse::button("middle")?;
    if layers
        .iter()
        .any(|layer| layer.values().any(|keys| keys.contains(&middle)))
        || macros.iter().any(|layer| {
            layer.values().flatten().any(
                |step| matches!(step,crate::macros::Step::Tap{keys,..} if keys.contains(&middle)),
            )
        })
    {
        for layer in &mut layers {
            layer
                .entry(crate::mouse::MIDDLE)
                .or_insert_with(|| vec![middle]);
        }
    }
    let thumb = match config.thumb {
        None => crate::layers::ThumbMode::Passthrough,
        Some(Thumb::Modifier { key }) => {
            let key = output(&key)?;
            if !crate::mapping::is_modifier(key) {
                return Err("Thumb modifier must be a modifier key".into());
            }
            for layer in &mut layers {
                layer.insert(crate::layers::THUMB, vec![key]);
            }
            crate::layers::ThumbMode::Passthrough
        }
        Some(Thumb::Momentary { layer }) => {
            if layer == 0 || layer >= layers.len() {
                return Err("Momentary target must be a configured non-base layer".into());
            }
            crate::layers::ThumbMode::Momentary(layer)
        }
        Some(Thumb::Cycle { layers: cycle }) => {
            if !(2..=3).contains(&cycle.len())
                || cycle[0] != 0
                || cycle.iter().any(|index| *index >= layers.len())
                || cycle
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != cycle.len()
            {
                return Err("Cycle needs 2–3 distinct configured layers, starting with 0".into());
            }
            crate::layers::ThumbMode::Cycle(cycle)
        }
    };
    Ok(crate::layers::Profile {
        layers,
        macros,
        thumb,
    })
}
pub fn load(path: &Path) -> Result<(String, crate::layers::Profile), String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    file.take(65537)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    let bindings = parse(&text)?;
    Ok((text, bindings))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macro_validation_bounds_and_middle_ownership() {
        let wrap = |steps: &str| {
            format!(
                r#"{{"schema_version":1,"mappings":{{"01":{{"type":"macro","steps":{steps}}}}}}}"#
            )
        };
        for steps in [
            "[]",
            r#"[{"type":"tap","keys":["A","A"]}]"#,
            r#"[{"type":"tap","keys":["A"],"hold_ms":0}]"#,
            r#"[{"type":"tap","keys":["NoSuchKey"]}]"#,
            r#"[{"type":"delay","ms":10001}]"#,
            r#"[{"type":"delay","ms":-1}]"#,
            r#"[{"type":"mouse_button","button":"unknown"}]"#,
            r#"[{"type":"tap","keys":["A"],"extra":true}]"#,
        ] {
            assert!(parse(&wrap(steps)).is_err(), "{steps}");
        }
        let too_many = format!("[{}]", vec![r#"{"type":"delay","ms":1}"#; 129].join(","));
        assert!(parse(&wrap(&too_many)).is_err());
        let too_long = format!("[{}]", vec![r#"{"type":"delay","ms":10000}"#; 7].join(","));
        assert!(parse(&wrap(&too_long)).is_err());
        let p=parse(&wrap(r#"[{"type":"mouse_button","button":"middle"},{"type":"mouse_wheel","direction":"up"},{"type":"tap","keys":["LeftCtrl","A"]}]"#)).unwrap();
        assert_eq!(p.macros[0][&control("01").unwrap()].len(), 3);
        assert_eq!(
            p.layers[0][&crate::mouse::MIDDLE],
            vec![crate::mouse::button("middle").unwrap()]
        );
    }
    #[test]
    fn mouse_actions_validate_and_native_middle_shares_across_layers() {
        let profile=parse(r#"{"schema_version":1,"mappings":{"01":{"type":"mouse_button","button":"middle"},"02":{"type":"mouse_wheel","direction":"left"}},"layers":[{"name":"Shifted"}]}"#).unwrap();
        let middle = crate::mouse::button("middle").unwrap();
        for layer in &profile.layers {
            assert_eq!(layer[&crate::mouse::MIDDLE], vec![middle]);
        }
        let mut engine = crate::mapping::Engine::new(profile.layers[0].clone());
        assert_eq!(engine.event(crate::mouse::MIDDLE, false).len(), 1);
        assert!(engine.event(control("01").unwrap(), false).is_empty());
        assert!(engine.event(control("01").unwrap(), true).is_empty());
        assert_eq!(
            engine.event(crate::mouse::MIDDLE, true),
            vec![crate::mapping::Output {
                key: middle,
                up: true
            }]
        );
        for action in [
            r#"{"type":"mouse_button","button":"unknown"}"#,
            r#"{"type":"mouse_wheel","direction":"diagonal"}"#,
            r#"{"type":"mouse_button","button":"left","extra":true}"#,
        ] {
            assert!(
                parse(&format!(
                    r#"{{"schema_version":1,"mappings":{{"01":{action}}}}}"#
                ))
                .is_err()
            );
        }
    }
    #[test]
    fn media_outputs_share_ownership_and_release_on_reload() {
        // Microsoft Keyboard Input Overview: Consumer controls, set-1 make codes.
        for (name, scan) in [
            ("VolumeMute", 0x20),
            ("VolumeDown", 0x2e),
            ("VolumeUp", 0x30),
            ("MediaPlayPause", 0x22),
            ("MediaStop", 0x24),
            ("MediaPreviousTrack", 0x10),
            ("MediaNextTrack", 0x19),
        ] {
            let key = output(name).unwrap();
            assert_eq!(key, Key { scan, extended: 2 });
            let text = format!(
                r#"{{"schema_version":1,"mappings":{{"01":{{"type":"key","key":"{name}"}},"02":{{"type":"key","key":"{name}"}}}}}}"#
            );
            let profile = parse(&text).unwrap();
            let mut engine = crate::mapping::Engine::new(profile.layers[0].clone());
            let first = control("01").unwrap();
            let second = control("02").unwrap();
            assert_eq!(
                engine.event(first, false),
                vec![crate::mapping::Output { key, up: false }]
            );
            assert!(engine.event(second, false).is_empty());
            assert!(engine.event(first, true).is_empty());
            assert_eq!(
                engine.replace(Bindings::new()),
                vec![crate::mapping::Output { key, up: true }]
            );
            assert!(engine.event(second, true).is_empty());
        }
    }
    #[test]
    fn validates_layer_targets_cycles_and_thumb_conflicts() {
        for text in [
            r#"{"schema_version":1,"thumb":{"mode":"momentary","layer":1}}"#,
            r#"{"schema_version":1,"layers":[{"name":"shifted"}],"thumb":{"mode":"cycle","layers":[0,0]}}"#,
            r#"{"schema_version":1,"layers":[{"name":"base"}]}"#,
            r#"{"schema_version":1,"thumb":{"mode":"modifier","key":"B"}}"#,
            r#"{"schema_version":1,"thumb":{"mode":"modifier","key":"scan:E0:2A"}}"#,
            r#"{"schema_version":1,"thumb":{"mode":"modifier"},"mappings":{"thumb":{"type":"disabled"}}}"#,
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        let profile = parse(r#"{"schema_version":1,"layers":[{"name":"shifted","mappings":{"wheel_up":{"type":"key","key":"B"}}}],"thumb":{"mode":"momentary","layer":1}}"#).unwrap();
        assert_eq!(profile.layers.len(), 2);
        assert!(profile.layers[0].is_empty());
        assert_eq!(profile.layers[1][&crate::mouse::WHEEL_UP].len(), 1);
        let modifier = parse(r#"{"schema_version":1,"thumb":{"mode":"modifier"}}"#).unwrap();
        assert_eq!(
            modifier.layers[0][&crate::layers::THUMB],
            vec![crate::layers::THUMB]
        );
    }
    #[test]
    fn validates_before_applying() {
        let b = parse(r#"{"schema_version":1,"mappings":{"01":{"type":"chord","keys":["LeftCtrl","B"]},"02":{"type":"disabled"}}}"#).unwrap();
        assert_eq!(b.layers[0][&control("01").unwrap()].len(), 2);
        assert!(b.layers[0][&control("02").unwrap()].is_empty());
        for invalid in [
            r#"{"schema_version":2,"mappings":{}}"#,
            r#"{"schema_version":1,"mappings":{"unknown_control":{"type":"disabled"}}}"#,
            r#"{"schema_version":1,"mappings":{"01":{"type":"chord","keys":["B","B"]}}}"#,
            r#"{"schema_version":1,"mappings":{},"typo":true}"#,
        ] {
            assert!(parse(invalid).is_err());
        }
    }
    #[test]
    fn twenty_independent_controls_and_extended_keys() {
        let controls: std::collections::BTreeSet<_> = (1..=20)
            .map(|n| control(&format!("{n:02}")).unwrap())
            .collect();
        assert_eq!(controls.len(), 20);
        assert_eq!(
            output("RightAlt").unwrap(),
            Key {
                scan: 0x38,
                extended: 2
            }
        );
        assert_eq!(output("scan:E0:48").unwrap(), output("Up").unwrap());
        assert!(control("21").is_none());
    }
    #[test]
    fn windows_utf8_bom_is_accepted_but_duplicate_controls_are_rejected() {
        assert!(parse("\u{feff}{\"schema_version\":1,\"mappings\":{}}").is_ok());
        assert!(parse(r#"{"schema_version":1,"mappings":{"01":{"type":"disabled"},"01":{"type":"key","key":"B"}}}"#).is_err());
    }
}
