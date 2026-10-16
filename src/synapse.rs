//! Conservative Synapse 3 conversion. Source archives remain passive metadata.
use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};
pub const MAX_ARCHIVE: usize = 16 * 1024;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceImport {
    pub format: String,
    pub filename: String,
    pub archive_hex: String,
    pub profile_id: String,
    pub keymap_id: String,
}
impl SourceImport {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "synapse3"
            || self.filename.len() > 128
            || self.archive_hex.is_empty()
            || self.archive_hex.len() > MAX_ARCHIVE * 2
            || !self.archive_hex.len().is_multiple_of(2)
            || !self.archive_hex.bytes().all(|b| b.is_ascii_hexdigit())
            || self.profile_id.len() > 128
            || self.keymap_id.len() > 128
        {
            return Err("Invalid preserved Synapse import metadata".into());
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct Candidate {
    pub name: String,
    pub selected_in_source: bool,
    pub config: Value,
    pub warnings: Vec<String>,
    pub converted: usize,
    pub inactive: usize,
    pub blocked: bool,
}
#[derive(Serialize)]
pub struct Preview {
    pub device: String,
    pub candidates: Vec<Candidate>,
    pub preserved_bytes: usize,
    pub notes: Vec<String>,
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}
fn value<'a, 'input>(node: Node<'a, 'input>, name: &str) -> &'a str {
    child(node, name)
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
}
fn xml(text: &str) -> Result<Document<'_>, String> {
    Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 30000,
            ..Default::default()
        },
    )
    .map_err(|e| format!("Invalid XML: {e}"))
}
fn known_fields(node: Node<'_, '_>, allowed: &[&str]) -> bool {
    let mut seen = BTreeSet::new();
    node.children()
        .filter(|n| n.is_element())
        .all(|n| allowed.contains(&n.tag_name().name()) && seen.insert(n.tag_name().name()))
}
fn input(node: Node<'_, '_>) -> Result<String, String> {
    match value(node, "InputType") {
        "KeyInput" => {
            let key = child(node, "KeyInput").ok_or("Missing key input")?;
            if !known_fields(key, &["HID_Page", "HID_Id"]) {
                return Err("Unrecognized physical key description".into());
            }
            if value(key, "HID_Page") != "7" {
                return Err("Unsupported input HID page".into());
            }
            let usage = value(key, "HID_Id")
                .parse::<u16>()
                .map_err(|_| "Invalid input usage")?;
            let uses = [
                30, 31, 32, 33, 34, 43, 20, 26, 8, 21, 57, 4, 22, 7, 9, 225, 29, 27, 6, 44,
            ];
            if let Some(i) = uses.iter().position(|u| *u == usage) {
                return Ok(format!("{:02}", i + 1));
            }
            if usage == 226 {
                return Ok("thumb".into());
            }
            Err(format!("Unrecognized physical input usage {usage}"))
        }
        "MouseInput" => match value(node, "MouseInput") {
            "ScrollUp" => Ok("wheel_up".into()),
            "ScrollDown" => Ok("wheel_down".into()),
            "ScrollButton" => Ok("wheel_click".into()),
            other => Err(format!("Unrecognized mouse input {other}")),
        },
        "DPADInput" => match value(node, "DPADInput") {
            "DIRECTION_UP" => Ok("dpad_up".into()),
            "DIRECTION_DOWN" => Ok("dpad_down".into()),
            "DIRECTION_LEFT" => Ok("dpad_left".into()),
            "DIRECTION_RIGHT" => Ok("dpad_right".into()),
            other => Err(format!("Unrecognized active D-pad input {other}")),
        },
        other => Err(format!("Unrecognized input type {other}")),
    }
}
fn keyboard(node: Node<'_, '_>) -> Result<Value, String> {
    let group = child(node, "KeyGroup").ok_or("Missing keyboard action")?;
    if !known_fields(group, &["KeyAssignment"])
        || group.children().filter(|n| n.is_element()).count() != 1
    {
        return Err("Unsupported keyboard repeat or sequence settings".into());
    }
    let key = child(group, "KeyAssignment").ok_or("Missing key assignment")?;
    if !known_fields(key, &["Scancode", "VirtualKey", "Modifier", "ExtendedKey"]) {
        return Err("Unsupported key assignment options".into());
    }
    let scan = value(key, "Scancode")
        .parse::<u16>()
        .map_err(|_| "Invalid output scan code")?;
    if !(1..=127).contains(&scan) {
        return Err("Output scan code is outside the supported range".into());
    }
    let mut keys = Vec::new();
    let mut extended = false;
    for modifier in value(key, "Modifier").split_whitespace() {
        let name = match modifier {
            "Left_Ctrl" => "LeftCtrl",
            "Right_Ctrl" => "RightCtrl",
            "Left_Shift" => "LeftShift",
            "Right_Shift" => "RightShift",
            "Left_Alt" => "LeftAlt",
            "Right_Alt" => "RightAlt",
            "Left_Win" => "LeftWin",
            "Right_Win" => "RightWin",
            "Extended_Key" => {
                extended = true;
                continue;
            }
            other => return Err(format!("Unsupported modifier {other}")),
        };
        keys.push(name.to_string());
    }
    let ext = value(key, "ExtendedKey");
    if (extended && ext != "Extended_00") || (!extended && !ext.is_empty()) {
        return Err("Unrecognized extended-key encoding".into());
    }
    let target = crate::mapping::Key {
        scan,
        extended: if extended { 2 } else { 0 },
    };
    let mut names: Vec<String> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .map(|c| c.to_string())
        .collect();
    names.extend(
        [
            "LeftCtrl",
            "RightCtrl",
            "LeftShift",
            "RightShift",
            "LeftAlt",
            "RightAlt",
            "LeftWin",
            "RightWin",
            "Space",
            "Tab",
            "Enter",
            "Escape",
            "Backspace",
            "Up",
            "Down",
            "Left",
            "Right",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Insert",
            "Delete",
            "Minus",
        ]
        .map(String::from),
    );
    keys.push(
        names
            .into_iter()
            .find(|n| crate::config::output(n).ok() == Some(target))
            .unwrap_or_else(|| format!("scan:{}{scan:04X}", if extended { "E0:" } else { "" })),
    );
    if keys.len() == 1 {
        Ok(json!({"type":"key","key":keys[0]}))
    } else {
        Ok(json!({"type":"chord","keys":keys}))
    }
}
fn action(node: Node<'_, '_>, control: &str) -> Result<Value, String> {
    if !known_fields(
        node,
        &[
            "IsDefault",
            "IsHyperShift",
            "MappingGroup",
            "InputType",
            "KeyInput",
            "MouseInput",
            "DPADInput",
            "KeyGroup",
            "MouseGroup",
            "WindowsShortcutGroup",
        ],
    ) {
        return Err("Unrecognized mapping options".into());
    }
    match value(node, "MappingGroup") {
        "Keyboard" => keyboard(node),
        "Mouse" => {
            let group = child(node, "MouseGroup").ok_or("Missing mouse assignment")?;
            if !known_fields(group, &["MouseAssignment"]) {
                return Err("Unsupported mouse assignment options".into());
            }
            let expected = match control {
                "wheel_up" => "ScrollUp",
                "wheel_down" => "ScrollDown",
                "wheel_click" => "ScrollClick",
                _ => "",
            };
            if !expected.is_empty() && value(group, "MouseAssignment") == expected {
                Ok(json!({"type":"passthrough"}))
            } else {
                Err("Mouse output actions are not implemented yet".into())
            }
        }
        "Win8Shortcuts" => {
            let group = child(node, "WindowsShortcutGroup").ok_or("Missing shortcut")?;
            if !known_fields(group, &["WindowsShortcutAssignment"]) {
                return Err("Unsupported shortcut options".into());
            }
            let key = match value(group, "WindowsShortcutAssignment") {
                "Copy" => "C",
                "Cut" => "X",
                "Paste" => "V",
                other => return Err(format!("Unsupported Windows shortcut {other}")),
            };
            Ok(json!({"type":"chord","keys":["LeftCtrl",key]}))
        }
        other => Err(format!("Action group {other:?} is not implemented")),
    }
}
pub fn preview(filename: &str, bytes: &[u8]) -> Result<Preview, String> {
    if bytes.len() > MAX_ARCHIVE {
        return Err("This first importer supports backups up to 16 KiB, so the original can be preserved within the 64 KiB JSON configuration limit".into());
    }
    if filename.len() > 128 {
        return Err("Backup filename is too long".into());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("Not a supported Synapse 3 ZIP backup: {e}"))?;
    if archive.len() > 128 {
        return Err("Archive contains too many entries".into());
    }
    let mut files = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|e| format!("Cannot read ZIP entry: {e}"))?;
        let name = file.name().map_err(|e| e.to_string())?.into_owned();
        if name.starts_with('/')
            || name.contains('\\')
            || name.contains(':')
            || name
                .trim_end_matches('/')
                .split('/')
                .any(|p| matches!(p, "" | "." | ".."))
            || !names.insert(name.to_lowercase())
        {
            return Err("Unsafe or duplicate archive entry name".into());
        }
        if file.encrypted() || file.size() > 512 * 1024 {
            return Err("Encrypted or oversized archive entry".into());
        }
        if file.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        file.take(512 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        total += data.len();
        if data.len() > 512 * 1024 || total > 1024 * 1024 {
            return Err("Archive expands beyond the importer limit".into());
        }
        if name.ends_with(".xml") {
            let text =
                String::from_utf8(data).map_err(|_| "Only UTF-8 XML backups are supported")?;
            xml(&text)?;
            files.insert(name, text);
        }
    }
    let info = xml(files
        .get("DeviceInfo.xml")
        .ok_or("Missing DeviceInfo.xml")?)?;
    let root = info.root_element();
    if value(root, "Vendor_ID") != "5426" || value(root, "Product_ID") != "555" {
        return Err(
            "Only Tartarus V2 (1532:022B) backups are supported; Tartarus Pro is excluded".into(),
        );
    }
    let archive_hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let mut candidates = Vec::new();
    for (path, text) in &files {
        let doc = xml(text)?;
        if doc.root_element().tag_name().name() != "Profile" || !path.starts_with("Profiles/") {
            continue;
        }
        let profile = doc.root_element();
        let id = value(profile, "ProfileId");
        if id.is_empty() || id.contains('/') || id.contains('\\') {
            return Err("Invalid profile identifier".into());
        }
        let prefix = format!("Features/{id}/");
        let mut mappings = None;
        for (p, t) in &files {
            if p.starts_with(&prefix) {
                let d = xml(t)?;
                if d.root_element().tag_name().name() == "Keymappings" {
                    if mappings.is_some() {
                        return Err("Ambiguous keymapping feature".into());
                    }
                    mappings = Some(t);
                }
            }
        }
        let Some(mapping_text) = mappings else {
            continue;
        };
        let mapping_doc = xml(mapping_text)?;
        let mapping_root = mapping_doc.root_element();
        for keymap in child(mapping_root, "KeymapList")
            .ok_or("Missing keymap list")?
            .children()
            .filter(|n| n.has_tag_name("Keymapping"))
        {
            if candidates.len() >= 16 {
                return Err("Too many keymaps for this importer".into());
            }
            let mut candidate = Candidate {
                name: format!("{} / {}", value(profile, "Name"), value(keymap, "Name")),
                selected_in_source: value(keymap, "Id") == value(mapping_root, "ActiveKeymapId"),
                config: json!({"schema_version":1,"mappings":{},"layers":[{"name":"HyperShift","mappings":{}}]}),
                warnings: Vec::new(),
                converted: 0,
                inactive: 0,
                blocked: false,
            };
            let mut base = serde_json::Map::new();
            let mut shifted = serde_json::Map::new();
            let mut thumb = [false; 2];
            let direction = value(keymap, "DPADMode") == "DIRECTION";
            if !direction {
                candidate.blocked = true;
                candidate
                    .warnings
                    .push("Only the confirmed DIRECTION D-pad mode can be imported".into());
            }
            let list = child(keymap, "MappingList").ok_or("Missing mapping list")?;
            if list
                .children()
                .any(|n| n.is_element() && !n.has_tag_name("Mapping"))
            {
                return Err("Unrecognized mapping-list entries".into());
            }
            if list.children().filter(|n| n.is_element()).count() > 128 {
                return Err("Too many mappings".into());
            }
            for m in list.children().filter(|n| n.has_tag_name("Mapping")) {
                if direction
                    && value(m, "InputType") == "DPADInput"
                    && value(m, "DPADInput").starts_with("BUTTON_")
                {
                    candidate.inactive += 1;
                    continue;
                }
                let control = match input(m) {
                    Ok(control) => control,
                    Err(e) => {
                        candidate.blocked = true;
                        candidate.warnings.push(e);
                        continue;
                    }
                };
                let hs = match value(m, "IsHyperShift") {
                    "" | "false" => false,
                    "true" => true,
                    _ => {
                        candidate.blocked = true;
                        candidate.warnings.push("Invalid HyperShift flag".into());
                        continue;
                    }
                };
                let bindings = if hs { &mut shifted } else { &mut base };
                if bindings.contains_key(&control) {
                    candidate.blocked = true;
                    candidate.warnings.push(format!(
                        "Duplicate assignment for {control} in the same layer"
                    ));
                    continue;
                }
                if control == "thumb" && value(m, "MappingGroup") == "HyperShift" {
                    if !known_fields(
                        m,
                        &[
                            "IsDefault",
                            "IsHyperShift",
                            "MappingGroup",
                            "InputType",
                            "KeyInput",
                        ],
                    ) {
                        candidate.blocked = true;
                        candidate
                            .warnings
                            .push("Unrecognized thumb trigger options".into());
                    }
                    thumb[usize::from(hs)] = true;
                    bindings.insert(control, json!({"type":"disabled"}));
                    candidate.converted += 1;
                    continue;
                }
                match action(m, &control) {
                    Ok(a) => {
                        bindings.insert(control, a);
                        candidate.converted += 1;
                    }
                    Err(e) => {
                        candidate.warnings.push(format!(
                            "{} / {control}: {e}; disabled in the converted draft",
                            if hs { "HyperShift" } else { "Base" }
                        ));
                        bindings.insert(control, json!({"type":"disabled"}));
                    }
                }
            }
            if thumb.iter().any(|t| *t) {
                // The current runtime has one global physical thumb trigger. Refuse conflicting source semantics.
                let both = thumb.iter().all(|t| *t);
                if !both {
                    candidate.blocked = true;
                    candidate.warnings.push(
                        "Thumb trigger differs across source layers; cannot reproduce safely"
                            .into(),
                    );
                }
                base.remove("thumb");
                shifted.remove("thumb");
                candidate.config["thumb"] = json!({"mode":"momentary","layer":1});
            }
            candidate.config["mappings"] = Value::Object(base);
            candidate.config["layers"][0]["mappings"] = Value::Object(shifted);
            candidate.config["source_import"] = serde_json::to_value(SourceImport {
                format: "synapse3".into(),
                filename: filename.into(),
                archive_hex: archive_hex.clone(),
                profile_id: id.into(),
                keymap_id: value(keymap, "Id").into(),
            })
            .map_err(|e| e.to_string())?;
            crate::config::parse(
                &serde_json::to_string_pretty(&candidate.config).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Converted profile cannot be represented: {e}"))?;
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() {
        return Err("No convertible profile keymaps found".into());
    }
    Ok(Preview{device:"Razer Tartarus V2 (1532:022B)".into(),candidates,preserved_bytes:bytes.len(),notes:vec!["Original archive is preserved byte-for-byte as passive source_import metadata, including inactive D-pad settings, lighting, and any macros. It is not executed.".into(),"Lighting, polling, application links and macros are not applied by this importer. Unsupported active assignments are disabled and listed below; unknown physical inputs block loading that keymap.".into(),"Conversion is one-way. Synapse export and universal Synapse-version compatibility are not claimed.".into()]})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn backup(mappings: &str, product: &str, extra: Option<(&str, &str)>) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let files = [
            (
                "DeviceInfo.xml".to_string(),
                format!(
                    "<Device><Vendor_ID>5426</Vendor_ID><Product_ID>{product}</Product_ID></Device>"
                ),
            ),
            (
                "Profiles/p.xml".into(),
                "<Profile><Name>Fixture</Name><ProfileId>p</ProfileId></Profile>".into(),
            ),
            (
                "Features/p/keys.xml".into(),
                format!(
                    "<Keymappings><KeymapList><Keymapping><Name>Custom</Name><Id>k</Id><MappingList>{mappings}</MappingList><DPADMode>DIRECTION</DPADMode></Keymapping></KeymapList><ActiveKeymapId>k</ActiveKeymapId></Keymappings>"
                ),
            ),
        ];
        for (name, text) in files
            .into_iter()
            .chain(extra.map(|(n, t)| (n.into(), t.into())))
        {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn key(usage: u16, scan: u16, modifiers: &str) -> String {
        format!(
            "<Mapping><MappingGroup>Keyboard</MappingGroup><InputType>KeyInput</InputType><KeyInput><HID_Page>7</HID_Page><HID_Id>{usage}</HID_Id></KeyInput><KeyGroup><KeyAssignment><Scancode>{scan}</Scancode>{modifiers}</KeyAssignment></KeyGroup></Mapping>"
        )
    }
    #[test]
    fn converts_chords_extended_keys_and_preserves_exact_archive() {
        let mappings = key(30, 46, "<Modifier>Left_Ctrl</Modifier>")
            + &key(
                31,
                83,
                "<Modifier>Extended_Key</Modifier><ExtendedKey>Extended_00</ExtendedKey>",
            );
        let bytes = backup(
            &mappings,
            "555",
            Some(("future.dat", "opaque inactive data")),
        );
        let result = preview("fixture.synapse3", &bytes).unwrap();
        let c = &result.candidates[0];
        assert!(c.selected_in_source);
        assert!(!c.blocked);
        assert_eq!(
            c.config["mappings"]["01"],
            json!({"type":"chord","keys":["LeftCtrl","C"]})
        );
        assert_eq!(c.config["mappings"]["02"]["key"], "Delete");
        let hex = c.config["source_import"]["archive_hex"].as_str().unwrap();
        let decoded: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(decoded, bytes);
        crate::config::parse(&serde_json::to_string(&c.config).unwrap()).unwrap();
    }
    #[test]
    fn excludes_inactive_joystick_entries_and_converts_thumb_trigger() {
        let mut mappings = String::from(
            "<Mapping><InputType>DPADInput</InputType><DPADInput>BUTTON_UPLEFT</DPADInput><MappingGroup>Joystick</MappingGroup></Mapping>",
        );
        for hs in [false, true] {
            mappings += &format!(
                "<Mapping><IsHyperShift>{hs}</IsHyperShift><MappingGroup>HyperShift</MappingGroup><InputType>KeyInput</InputType><KeyInput><HID_Page>7</HID_Page><HID_Id>226</HID_Id></KeyInput></Mapping>"
            );
        }
        let p = preview("test", &backup(&mappings, "555", None)).unwrap();
        let c = &p.candidates[0];
        assert_eq!(c.inactive, 1);
        assert!(!c.blocked);
        assert_eq!(c.config["thumb"], json!({"mode":"momentary","layer":1}));
        assert!(c.config["mappings"].get("thumb").is_none());
    }
    #[test]
    fn rejects_other_models_unsafe_archives_and_dtd() {
        assert!(preview("test", &backup("", "580", None)).is_err());
        assert!(preview("test", &backup("", "555", Some(("../bad.xml", "<bad/>")))).is_err());
        assert!(
            preview(
                "test",
                &backup(
                    "",
                    "555",
                    Some(("extra.xml", "<!DOCTYPE x [<!ENTITY e 'expand'>]><x>&e;</x>"))
                )
            )
            .is_err()
        );
        assert!(preview("test", &vec![0; MAX_ARCHIVE + 1]).is_err());
    }
    #[test]
    fn unsupported_outputs_are_disabled_unknown_inputs_and_duplicates_block() {
        let unsupported = "<Mapping><InputType>KeyInput</InputType><KeyInput><HID_Page>7</HID_Page><HID_Id>30</HID_Id></KeyInput><MappingGroup>Macro</MappingGroup></Mapping>";
        let p = preview("test", &backup(unsupported, "555", None)).unwrap();
        assert_eq!(p.candidates[0].config["mappings"]["01"]["type"], "disabled");
        assert_eq!(p.candidates[0].warnings.len(), 1);
        let p = preview("test", &backup(&key(999, 30, ""), "555", None)).unwrap();
        assert!(p.candidates[0].blocked);
        let mapping = key(30, 30, "");
        let p = preview("test", &backup(&(mapping.clone() + &mapping), "555", None)).unwrap();
        assert!(p.candidates[0].blocked);
        let p = preview(
            "test",
            &backup(&key(30, 30, "<Turbo>true</Turbo>"), "555", None),
        )
        .unwrap();
        assert_eq!(p.candidates[0].config["mappings"]["01"]["type"], "disabled");
    }
}
