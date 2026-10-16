//! Pure keyboard ownership engine. All keys here belong to the selected keypad.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub scan: u16,
    pub extended: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub key: Key,
    pub up: bool,
}
pub type Bindings = BTreeMap<Key, Vec<Key>>;

#[derive(Default)]
pub struct Engine {
    bindings: Bindings,
    pressed: BTreeMap<Key, Vec<Key>>,
    owners: BTreeMap<Key, usize>,
    quarantine: BTreeSet<Key>,
}
impl Engine {
    pub fn new(bindings: Bindings) -> Self {
        Self {
            bindings,
            ..Self::default()
        }
    }
    pub fn event(&mut self, input: Key, up: bool) -> Vec<Output> {
        if self.quarantine.contains(&input) {
            if up {
                self.quarantine.remove(&input);
            }
            return vec![];
        }
        if up {
            let Some(outputs) = self.pressed.remove(&input) else {
                // Startup may observe the release of a key pressed before takeover.
                return if self.owners.contains_key(&input) {
                    vec![]
                } else {
                    vec![Output {
                        key: input,
                        up: true,
                    }]
                };
            };
            return self.release(outputs);
        }
        if let Some(outputs) = self.pressed.get(&input) {
            // A repeated physical down repeats the last non-modifier output only.
            return outputs
                .last()
                .filter(|k| repeatable(**k))
                .map(|key| {
                    vec![Output {
                        key: *key,
                        up: false,
                    }]
                })
                .unwrap_or_default();
        }
        let outputs = self
            .bindings
            .get(&input)
            .cloned()
            .unwrap_or_else(|| vec![input]);
        let mut events = vec![];
        for key in &outputs {
            if crate::mouse::is_wheel(*key) {
                events.push(Output {
                    key: *key,
                    up: false,
                });
                continue;
            }
            let count = self.owners.entry(*key).or_default();
            if *count == 0 {
                events.push(Output {
                    key: *key,
                    up: false,
                });
            }
            *count += 1;
        }
        self.pressed.insert(input, outputs);
        events
    }
    /// Synthetic owners share the exact same reference counts as physical mappings.
    pub fn virtual_press(&mut self, owner: Key, keys: Vec<Key>) -> Vec<Output> {
        debug_assert!(owner.extended & 0x8000 != 0);
        self.bindings.insert(owner, keys);
        let events = self.event(owner, false);
        self.bindings.remove(&owner);
        events
    }
    fn release(&mut self, keys: Vec<Key>) -> Vec<Output> {
        let mut events = vec![];
        for key in keys.into_iter().rev() {
            if crate::mouse::is_wheel(key) {
                continue;
            }
            let count = self
                .owners
                .get_mut(&key)
                .expect("pressed outputs have owners");
            *count -= 1;
            if *count == 0 {
                self.owners.remove(&key);
                events.push(Output { key, up: true });
            }
        }
        events
    }
    pub fn replace(&mut self, bindings: Bindings) -> Vec<Output> {
        self.quarantine.extend(
            self.pressed
                .keys()
                .copied()
                .filter(|k| k.extended & 0x8000 == 0),
        );
        let releases = self.release_all();
        self.bindings = bindings;
        releases
    }
    /// A wheel notch is an impulse. Repeat its final key even if another action
    /// owns that key, while never releasing the other action's hold.
    pub fn pulse(&mut self, input: Key) -> Vec<Output> {
        let mut result = self.event(input, false);
        if let Some(last) = self
            .pressed
            .get(&input)
            .and_then(|keys| keys.last())
            .copied()
            && repeatable(last)
            && !result.iter().any(|event| event.key == last && !event.up)
        {
            result.push(Output {
                key: last,
                up: false,
            });
        }
        result.extend(self.event(input, true));
        result
    }
    pub fn release_all(&mut self) -> Vec<Output> {
        let mut releases = Vec::new();
        for keys in std::mem::take(&mut self.pressed).into_values() {
            releases.extend(self.release(keys));
        }
        releases
    }
    pub fn owned_count(&self) -> usize {
        self.owners.len()
    }
    pub fn clear_quarantine(&mut self, input: Key) {
        self.quarantine.remove(&input);
    }
    pub fn quarantined(&self, input: Key) -> bool {
        self.quarantine.contains(&input)
    }
}

pub fn is_modifier(key: Key) -> bool {
    matches!(
        (key.scan, key.extended),
        (0x1d | 0x38, 0 | 2) | (0x2a | 0x36, 0) | (0x5b | 0x5c, 2)
    )
}
fn repeatable(key: Key) -> bool {
    !is_modifier(key) && !crate::mouse::is_button(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mouse_buttons_do_not_repeat_or_release_another_owner() {
        let mouse = crate::mouse::button("left").unwrap();
        let mut engine = Engine::new([(key(2), vec![mouse]), (key(3), vec![mouse])].into());
        assert_eq!(
            engine.event(key(2), false),
            vec![Output {
                key: mouse,
                up: false
            }]
        );
        assert!(engine.event(key(2), false).is_empty());
        assert!(engine.pulse(key(3)).is_empty());
        assert_eq!(engine.owned_count(), 1);
        assert_eq!(
            engine.replace(Bindings::new()),
            vec![Output {
                key: mouse,
                up: true
            }]
        );
        assert!(engine.event(key(2), true).is_empty());
        assert_eq!(engine.owned_count(), 0);
    }
    #[test]
    fn wheel_outputs_are_independent_impulses_without_release_or_ownership() {
        let wheel = crate::mouse::wheel("down").unwrap();
        let mut engine = Engine::new([(key(2), vec![wheel]), (key(3), vec![wheel])].into());
        for input in [key(2), key(2), key(3)] {
            assert_eq!(
                engine.event(input, false),
                vec![Output {
                    key: wheel,
                    up: false
                }]
            );
        }
        assert_eq!(engine.owned_count(), 0);
        assert!(engine.release_all().is_empty());
        assert_eq!(
            engine.pulse(key(2)),
            vec![Output {
                key: wheel,
                up: false
            }]
        );
        assert!(engine.replace(Bindings::new()).is_empty());
    }
    #[test]
    fn extended_dpad_roll_preserves_independent_diagonal_holds() {
        let up = Key {
            scan: 0x48,
            extended: 2,
        };
        let right = Key {
            scan: 0x4d,
            extended: 2,
        };
        let mut e = Engine::new([(up, vec![key(0x1e)]), (right, vec![key(0x20)])].into());
        assert_eq!(e.event(up, false), vec![out(0x1e, false)]);
        assert_eq!(e.event(right, false), vec![out(0x20, false)]);
        assert_eq!(e.owned_count(), 2);
        assert_eq!(e.event(up, true), vec![out(0x1e, true)]);
        assert_eq!(e.event(right, true), vec![out(0x20, true)]);
        assert_eq!(e.owned_count(), 0);
    }
    fn key(scan: u16) -> Key {
        Key { scan, extended: 0 }
    }
    fn out(scan: u16, up: bool) -> Output {
        Output { key: key(scan), up }
    }
    #[test]
    fn shared_outputs_have_one_down_and_final_up() {
        let mut e = Engine::new([(key(2), vec![key(0x30)]), (key(3), vec![key(0x30)])].into());
        assert_eq!(e.event(key(2), false), vec![out(0x30, false)]);
        assert!(e.event(key(3), false).is_empty());
        assert!(e.event(key(2), true).is_empty());
        assert_eq!(e.event(key(3), true), vec![out(0x30, true)]);
        assert_eq!(e.owned_count(), 0);
    }
    #[test]
    fn wheel_pulse_repeats_shared_key_without_releasing_other_owner() {
        let mut e = Engine::new(
            [
                (key(2), vec![key(0x30)]),
                (crate::mouse::WHEEL_UP, vec![key(0x30)]),
            ]
            .into(),
        );
        e.event(key(2), false);
        assert_eq!(e.pulse(crate::mouse::WHEEL_UP), vec![out(0x30, false)]);
        assert_eq!(e.owned_count(), 1);
        assert_eq!(e.event(key(2), true), vec![out(0x30, true)]);
    }
    #[test]
    fn reload_releases_old_chord_and_quarantines_until_release() {
        let mut e = Engine::new([(key(2), vec![key(0x1d), key(0x30)])].into());
        assert_eq!(
            e.event(key(2), false),
            vec![out(0x1d, false), out(0x30, false)]
        );
        assert_eq!(
            e.replace([(key(2), vec![key(0x2e)])].into()),
            vec![out(0x30, true), out(0x1d, true)]
        );
        assert!(e.event(key(2), false).is_empty());
        assert!(e.event(key(2), true).is_empty());
        assert_eq!(e.event(key(2), false), vec![out(0x2e, false)]);
    }
    #[test]
    fn disabled_and_overlapping_passthrough_do_not_release_a_shared_key() {
        let mut e = Engine::new([(key(2), vec![]), (key(3), vec![key(0x30)])].into());
        assert!(e.event(key(2), false).is_empty());
        assert!(e.event(key(2), true).is_empty());
        assert_eq!(e.event(key(3), false), vec![out(0x30, false)]);
        assert!(e.event(key(0x30), false).is_empty());
        assert!(e.event(key(3), true).is_empty());
        assert_eq!(e.event(key(0x30), true), vec![out(0x30, true)]);
    }
    #[test]
    fn repeats_and_shutdown_preserve_shared_modifiers() {
        let mut e = Engine::new(
            [
                (key(2), vec![key(0x1d), key(0x30)]),
                (key(3), vec![key(0x1d), key(0x2e)]),
            ]
            .into(),
        );
        e.event(key(2), false);
        e.event(key(3), false);
        assert_eq!(e.event(key(2), false), vec![out(0x30, false)]);
        let up = e.release_all();
        assert_eq!(up.iter().filter(|o| o.key.scan == 0x1d).count(), 1);
        assert!(up.iter().all(|o| o.up));
        assert_eq!(e.owned_count(), 0);
    }
}
