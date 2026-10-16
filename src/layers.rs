//! Layer transitions consume only the selected keypad's captured thumb control.
use crate::mapping::{Bindings, Engine, Key, Output};
pub const THUMB: Key = Key {
    scan: 0x38,
    extended: 0,
};
#[derive(Clone, Debug, Default)]
pub enum ThumbMode {
    #[default]
    Passthrough,
    Momentary(usize),
    Cycle(Vec<usize>),
}
#[derive(Clone)]
pub struct Profile {
    pub layers: Vec<Bindings>,
    pub macros: Vec<crate::macros::Bindings>,
    pub thumb: ThumbMode,
}
pub struct LayerEngine {
    profile: Profile,
    engine: Engine,
    current: usize,
    previous: Option<usize>,
    thumb_held: bool,
    thumb_quarantine: bool,
    macros: crate::macros::Player,
    macro_held: std::collections::BTreeSet<Key>,
}
impl LayerEngine {
    pub fn new(profile: Profile) -> Self {
        let engine = Engine::new(profile.layers[0].clone());
        Self {
            profile,
            engine,
            current: 0,
            previous: None,
            thumb_held: false,
            thumb_quarantine: false,
            macros: Default::default(),
            macro_held: Default::default(),
        }
    }
    pub fn active(&self) -> usize {
        self.current
    }
    pub fn thumb_held(&self) -> bool {
        self.thumb_held
    }
    pub fn bindings(&self) -> &Bindings {
        &self.profile.layers[self.current]
    }
    fn transition(&mut self, layer: usize) -> Vec<Output> {
        self.macros.clear();
        self.current = layer;
        self.engine.replace(self.profile.layers[layer].clone())
    }
    pub fn event(&mut self, input: Key, up: bool) -> Vec<Output> {
        // Retain held triggers across transitions so repeats cannot start a new macro.
        if up {
            self.macro_held.remove(&input);
        }
        if let Some(steps) = self.profile.macros[self.current].get(&input) {
            if input == THUMB {
                self.thumb_held = !up;
            }
            let blocked = self.engine.quarantined(input);
            let mut outputs = self.engine.event(input, up);
            if !up && !blocked && self.macro_held.insert(input) {
                self.macros.start(input, steps, std::time::Instant::now());
                outputs.extend(self.tick(std::time::Instant::now()));
            }
            return outputs;
        }
        if input != THUMB {
            return self.engine.event(input, up);
        }
        let was_held = self.thumb_held;
        self.thumb_held = !up;
        if self.thumb_quarantine {
            if up {
                self.thumb_quarantine = false;
                self.engine.clear_quarantine(THUMB);
            }
            return vec![];
        }
        match self.profile.thumb.clone() {
            ThumbMode::Passthrough => self.engine.event(input, up),
            ThumbMode::Momentary(layer) => {
                if !up && !was_held {
                    self.previous = Some(self.current);
                    self.transition(layer)
                } else if up && was_held {
                    let previous = self.previous.take().unwrap_or(0);
                    self.transition(previous)
                } else {
                    vec![]
                }
            }
            ThumbMode::Cycle(layers) => {
                if !up && !was_held {
                    let position = layers
                        .iter()
                        .position(|layer| *layer == self.current)
                        .unwrap_or(0);
                    self.transition(layers[(position + 1) % layers.len()])
                } else {
                    vec![]
                }
            }
        }
    }
    pub fn pulse(&mut self, input: Key) -> Vec<Output> {
        if let Some(steps) = self.profile.macros[self.current].get(&input) {
            self.macros.start(input, steps, std::time::Instant::now());
            return self.tick(std::time::Instant::now());
        }
        self.engine.pulse(input)
    }
    pub fn tick(&mut self, now: std::time::Instant) -> Vec<Output> {
        self.macros.tick(&mut self.engine, now)
    }
    pub fn wait_ms(&self) -> u32 {
        self.macros.wait_ms(std::time::Instant::now())
    }
    pub fn replace(&mut self, profile: Profile) -> Vec<Output> {
        self.macros.clear();
        self.thumb_quarantine |= self.thumb_held;
        self.current = 0;
        self.previous = None;
        let releases = self.engine.replace(profile.layers[0].clone());
        self.profile = profile;
        releases
    }
    pub fn release_all(&mut self) -> Vec<Output> {
        self.macros.clear();
        self.engine.release_all()
    }
    pub fn owned_count(&self) -> usize {
        self.engine.owned_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reload_from_held_thumb_macro_requires_release_before_layer_switch() {
        let original=crate::config::parse(r#"{"schema_version":1,"mappings":{"thumb":{"type":"macro","steps":[{"type":"tap","keys":["A"],"hold_ms":10000}]}}}"#).unwrap();
        let next=crate::config::parse(r#"{"schema_version":1,"layers":[{"name":"Shifted"}],"thumb":{"mode":"momentary","layer":1}}"#).unwrap();
        let mut e = LayerEngine::new(original);
        assert_eq!(e.event(THUMB, false).len(), 1);
        assert_eq!(e.replace(next).len(), 1);
        assert!(e.event(THUMB, false).is_empty());
        assert_eq!(e.active(), 0);
        e.event(THUMB, true);
        e.event(THUMB, false);
        assert_eq!(e.active(), 1);
    }
    #[test]
    fn macros_cancel_on_layer_reload_and_shutdown_without_repeat_restart() {
        let text = r#"{"schema_version":1,"mappings":{"01":{"type":"macro","steps":[{"type":"tap","keys":["LeftCtrl","A"],"hold_ms":10000}]}},"layers":[{"name":"Shifted","mappings":{"01":{"type":"macro","steps":[{"type":"tap","keys":["B"],"hold_ms":10000}]}}}],"thumb":{"mode":"momentary","layer":1}}"#;
        let p = crate::config::parse(text).unwrap();
        let input = crate::config::control("01").unwrap();
        let mut e = LayerEngine::new(p.clone());
        assert_eq!(e.event(input, false).len(), 2);
        assert!(e.event(input, false).is_empty());
        assert_eq!(e.event(THUMB, false).len(), 2);
        assert_eq!(e.owned_count(), 0);
        assert!(e.event(input, false).is_empty());
        assert!(e.event(input, true).is_empty());
        assert_eq!(
            e.event(input, false)[0].key,
            crate::config::output("B").unwrap()
        );
        assert_eq!(e.replace(p.clone()).len(), 1);
        assert!(e.event(input, false).is_empty());
        e.event(input, true);
        assert_eq!(e.event(input, false).len(), 2);
        assert_eq!(e.release_all().len(), 2);
        assert!(
            e.tick(std::time::Instant::now() + std::time::Duration::from_secs(60))
                .is_empty()
        );
        assert_eq!(e.owned_count(), 0);
    }
    #[test]
    fn wheel_macro_is_an_impulse_and_does_not_stack() {
        let p=crate::config::parse(r#"{"schema_version":1,"mappings":{"wheel_up":{"type":"macro","steps":[{"type":"tap","keys":["A"],"hold_ms":40}]}}}"#).unwrap();
        let mut e = LayerEngine::new(p);
        assert_eq!(e.pulse(crate::mouse::WHEEL_UP).len(), 1);
        assert!(e.pulse(crate::mouse::WHEEL_UP).is_empty());
        assert_eq!(
            e.tick(std::time::Instant::now() + std::time::Duration::from_secs(1))
                .len(),
            1
        );
        assert_eq!(e.pulse(crate::mouse::WHEEL_UP).len(), 1);
        assert_eq!(e.release_all().len(), 1);
    }
    #[test]
    fn layer_change_releases_mouse_hold_and_quarantines_until_release() {
        let mouse = crate::mouse::button("right").unwrap();
        let mut p = profile(ThumbMode::Momentary(1));
        p.layers[0].insert(key(2), vec![mouse]);
        let mut engine = LayerEngine::new(p);
        assert_eq!(
            engine.event(key(2), false),
            vec![Output {
                key: mouse,
                up: false
            }]
        );
        assert_eq!(
            engine.event(THUMB, false),
            vec![Output {
                key: mouse,
                up: true
            }]
        );
        assert!(engine.event(key(2), false).is_empty());
        assert!(engine.event(key(2), true).is_empty());
        assert_eq!(engine.owned_count(), 0);
        assert_eq!(engine.event(key(2), false)[0].key, key(0x30));
        assert_eq!(engine.release_all().len(), 1);
    }
    fn key(scan: u16) -> Key {
        Key { scan, extended: 0 }
    }
    fn profile(thumb: ThumbMode) -> Profile {
        Profile {
            macros: vec![Default::default(); 3],
            layers: vec![
                [(key(2), vec![key(0x1e)])].into(),
                [(key(2), vec![key(0x30)])].into(),
                [(key(2), vec![key(0x2e)])].into(),
            ],
            thumb,
        }
    }
    #[test]
    fn momentary_releases_old_key_and_requires_fresh_press() {
        let mut e = LayerEngine::new(profile(ThumbMode::Momentary(1)));
        e.event(key(2), false);
        assert_eq!(
            e.event(THUMB, false),
            vec![Output {
                key: key(0x1e),
                up: true
            }]
        );
        assert_eq!(e.active(), 1);
        assert!(e.event(key(2), false).is_empty());
        assert!(e.event(key(2), true).is_empty());
        assert_eq!(e.event(key(2), false)[0].key, key(0x30));
        assert_eq!(
            e.event(THUMB, true),
            vec![Output {
                key: key(0x30),
                up: true
            }]
        );
        assert_eq!(e.active(), 0);
        assert!(e.event(key(2), true).is_empty());
        assert_eq!(e.owned_count(), 0);
    }
    #[test]
    fn cycle_ignores_repeat_and_advances_once_per_press() {
        let mut e = LayerEngine::new(profile(ThumbMode::Cycle(vec![0, 1, 2])));
        for expected in [1, 2, 0, 1] {
            e.event(THUMB, false);
            assert_eq!(e.active(), expected);
            e.event(THUMB, false);
            assert_eq!(e.active(), expected);
            e.event(THUMB, true);
            assert_eq!(e.active(), expected);
        }
    }
    #[test]
    fn reload_during_thumb_hold_does_not_emit_phantom_alt_or_skip_next_press() {
        for (old, new) in [
            (ThumbMode::Passthrough, ThumbMode::Momentary(1)),
            (ThumbMode::Momentary(1), ThumbMode::Passthrough),
            (ThumbMode::Passthrough, ThumbMode::Passthrough),
        ] {
            let mut e = LayerEngine::new(profile(old));
            e.event(THUMB, false);
            e.replace(profile(new));
            assert!(e.event(THUMB, true).is_empty());
            let outputs = e.event(THUMB, false);
            if matches!(e.profile.thumb, ThumbMode::Passthrough) {
                assert_eq!(outputs[0].key, THUMB);
            } else {
                assert_eq!(e.active(), 1);
            }
        }
    }
    #[test]
    fn modifier_mode_and_wheel_bindings_remain_layer_specific() {
        let mut p = profile(ThumbMode::Passthrough);
        p.layers[0].insert(THUMB, vec![key(0x1d)]);
        let mut e = LayerEngine::new(p);
        assert_eq!(
            e.event(THUMB, false)[0],
            Output {
                key: key(0x1d),
                up: false
            }
        );
        assert_eq!(
            e.event(THUMB, true)[0],
            Output {
                key: key(0x1d),
                up: true
            }
        );
        assert_eq!(e.owned_count(), 0);
    }
}
