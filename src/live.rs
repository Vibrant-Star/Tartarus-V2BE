//! Bounded in-memory live snapshots. No disk writes and no input-to-browser queues.
use crate::{
    config,
    layers::{Profile, ThumbMode},
    mapping::{Bindings, Key},
    mouse,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    net::TcpStream,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
pub const CONTROLS: [&str; 28] = [
    "01",
    "02",
    "03",
    "04",
    "05",
    "06",
    "07",
    "08",
    "09",
    "10",
    "11",
    "12",
    "13",
    "14",
    "15",
    "16",
    "17",
    "18",
    "19",
    "20",
    "thumb",
    "dpad_up",
    "dpad_down",
    "dpad_left",
    "dpad_right",
    "wheel_up",
    "wheel_down",
    "wheel_click",
];
#[derive(Clone, Default, Serialize)]
pub struct Frame {
    pub running: bool,
    pub layer: usize,
    pub layer_names: Vec<String>,
    pub held: BTreeSet<String>,
    pub presses: BTreeMap<String, u64>,
    pub last_control: Option<String>,
    pub last_action: Option<String>,
}
#[derive(Clone, Default, Serialize)]
struct Snapshot {
    sequence: u64,
    frame: Frame,
}
#[derive(Default)]
pub struct Hub {
    snapshot: Mutex<Snapshot>,
    changed: Condvar,
    viewers: AtomicUsize,
    closed: AtomicBool,
}
impl Hub {
    pub fn publish(&self, frame: &Frame) {
        if self.viewers.load(Ordering::Relaxed) == 0 {
            return;
        }
        let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        s.sequence = s.sequence.wrapping_add(1);
        s.frame = frame.clone();
        drop(s);
        self.changed.notify_all();
    }
    pub fn subscribe(self: &Arc<Self>) -> Option<Subscription> {
        self.viewers
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < 4).then_some(n + 1)
            })
            .ok()?;
        Some(Subscription(self.clone()))
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.changed.notify_all();
    }
    pub fn viewers(&self) -> usize {
        self.viewers.load(Ordering::Relaxed)
    }
}
pub struct Subscription(Arc<Hub>);
impl Drop for Subscription {
    fn drop(&mut self) {
        self.0.viewers.fetch_sub(1, Ordering::SeqCst);
    }
}
pub struct Close(pub Arc<Hub>);
impl Drop for Close {
    fn drop(&mut self) {
        self.0.close();
    }
}
impl Subscription {
    pub fn stream(self, mut socket: TcpStream) {
        if socket
            .set_write_timeout(Some(Duration::from_secs(2)))
            .is_err()
        {
            return;
        }
        if write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n").is_err(){return;}
        let mut previous = None;
        loop {
            let mut state = self.0.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            if previous == Some(state.sequence) && !self.0.closed.load(Ordering::SeqCst) {
                state = self
                    .0
                    .changed
                    .wait_timeout(state, Duration::from_secs(15))
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            if self.0.closed.load(Ordering::SeqCst) {
                break;
            }
            let snapshot = state.clone();
            drop(state);
            if previous == Some(snapshot.sequence) {
                if socket.write_all(b": keep-alive\n\n").is_err() {
                    break;
                }
            } else {
                let Ok(json) = serde_json::to_string(&snapshot) else {
                    break;
                };
                if writeln!(socket, "data: {json}\n").is_err() {
                    break;
                }
                previous = Some(snapshot.sequence);
            }
        }
    }
}
pub struct Physical {
    pub frame: Frame,
    labels: Vec<BTreeMap<Key, String>>,
    thumb: Option<String>,
}
impl Physical {
    pub fn new(profile: &Profile, text: &str) -> Self {
        let named: Vec<String> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .map(|c| c.to_string())
            .chain(
                [
                    "LeftCtrl",
                    "RightCtrl",
                    "LeftAlt",
                    "RightAlt",
                    "LeftShift",
                    "RightShift",
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
                    "VolumeMute",
                    "VolumeDown",
                    "VolumeUp",
                    "MediaPlayPause",
                    "MediaStop",
                    "MediaPreviousTrack",
                    "MediaNextTrack",
                ]
                .map(String::from),
            )
            .collect();
        let labels = profile
            .layers
            .iter()
            .enumerate()
            .map(|(index, bindings)| {
                bindings
                    .iter()
                    .map(|(input, keys)| {
                        let label = if let Some(steps) = profile.macros[index].get(input) {
                            format!("Macro · {} steps", steps.len())
                        } else if keys.is_empty() {
                            "Disabled".into()
                        } else {
                            keys.iter()
                                .map(|key| {
                                    if let Some(label) = crate::mouse::label(*key) {
                                        return label.to_string();
                                    }
                                    named
                                        .iter()
                                        .find(|n| config::output(n).ok() == Some(*key))
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            format!(
                                                "scan:{}{:04X}",
                                                if key.extended == 2 { "E0:" } else { "" },
                                                key.scan
                                            )
                                        })
                                })
                                .collect::<Vec<_>>()
                                .join(" + ")
                        };
                        (*input, label)
                    })
                    .collect()
            })
            .collect();
        let value: serde_json::Value =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_default();
        let mut layer_names = vec!["Base".into()];
        if let Some(layers) = value["layers"].as_array() {
            for layer in layers {
                layer_names.push(layer["name"].as_str().unwrap_or("Layer").into());
            }
        }
        let thumb = match &profile.thumb {
            ThumbMode::Passthrough => None,
            ThumbMode::Momentary(layer) => Some(format!(
                "Hold → {}",
                layer_names
                    .get(*layer)
                    .map(String::as_str)
                    .unwrap_or("layer")
            )),
            ThumbMode::Cycle(_) => Some("Cycle layers".into()),
        };
        Self {
            frame: Frame {
                running: true,
                layer_names,
                ..Default::default()
            },
            labels,
            thumb,
        }
    }
    pub fn event(&mut self, key: Key, up: bool) {
        let Some(name) = CONTROLS
            .iter()
            .find(|name| config::control(name) == Some(key))
        else {
            return;
        };
        if matches!(key, mouse::WHEEL_UP | mouse::WHEEL_DOWN) {
            if up {
                return;
            }
        } else if up {
            self.frame.held.remove(*name);
            return;
        } else if !self.frame.held.insert((*name).into()) {
            return;
        }
        *self.frame.presses.entry((*name).into()).or_default() += 1;
        self.frame.last_control = Some((*name).into());
        self.frame.last_action = Some(if key == crate::layers::THUMB && self.thumb.is_some() {
            self.thumb.clone().unwrap()
        } else {
            self.labels
                .get(self.frame.layer)
                .and_then(|l| l.get(&key))
                .cloned()
                .unwrap_or_else(|| "Native".into())
        });
    }
    pub fn mouse_bindings() -> Bindings {
        [mouse::WHEEL_UP, mouse::WHEEL_DOWN, mouse::MIDDLE]
            .into_iter()
            .map(|key| (key, vec![]))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn applied_layer_labels_and_unknown_keys_do_not_invent_controls() {
        let text = r#"{"schema_version":1,"mappings":{"01":{"type":"key","key":"A"}},"layers":[{"name":"Shifted","mappings":{"01":{"type":"key","key":"B"}}}],"thumb":{"mode":"momentary","layer":1}}"#;
        let mut physical = Physical::new(&config::parse(text).unwrap(), text);
        physical.event(
            Key {
                scan: 0x7f,
                extended: 0,
            },
            false,
        );
        assert!(physical.frame.held.is_empty());
        physical.event(crate::layers::THUMB, false);
        assert_eq!(
            physical.frame.last_action.as_deref(),
            Some("Hold → Shifted")
        );
        physical.frame.layer = 1;
        physical.event(config::control("01").unwrap(), false);
        assert_eq!(physical.frame.last_action.as_deref(), Some("B"));
        assert_eq!(physical.frame.layer_names, vec!["Base", "Shifted"]);
    }
    #[test]
    fn physical_state_retains_holds_ignores_repeats_and_tracks_native_mouse() {
        let p = Profile {
            macros: vec![Default::default()],
            layers: vec![Bindings::new()],
            thumb: ThumbMode::Passthrough,
        };
        let mut physical = Physical::new(&p, "{}");
        let key = config::control("01").unwrap();
        physical.event(key, false);
        physical.event(key, false);
        assert_eq!(physical.frame.presses["01"], 1);
        assert!(physical.frame.held.contains("01"));
        physical.event(key, true);
        assert!(physical.frame.held.is_empty());
        assert_eq!(physical.frame.last_action.as_deref(), Some("Native"));
        let packet = mouse::MouseStroke {
            state: 0x410,
            rolling: 240,
            ..Default::default()
        };
        for (key, up) in mouse::MouseMapper::default()
            .split(packet, &Physical::mouse_bindings())
            .1
        {
            physical.event(key, up);
        }
        assert!(physical.frame.held.contains("wheel_click"));
        assert!(!physical.frame.held.contains("wheel_up"));
        assert_eq!(physical.frame.presses["wheel_up"], 2);
    }
    #[test]
    fn slow_viewers_have_bounded_slots_and_latest_snapshot_only() {
        let hub = Arc::new(Hub::default());
        let slots: Vec<_> = (0..4).map(|_| hub.subscribe().unwrap()).collect();
        assert!(hub.subscribe().is_none());
        for layer in 0..1000 {
            hub.publish(&Frame {
                layer,
                ..Default::default()
            });
        }
        assert_eq!(hub.snapshot.lock().unwrap().frame.layer, 999);
        drop(slots);
        assert_eq!(hub.viewers(), 0);
    }
}
