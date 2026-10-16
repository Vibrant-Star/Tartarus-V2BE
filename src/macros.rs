//! Bounded, balanced sequences, advanced by the input thread without sleeping it.
use crate::mapping::{Engine, Key, Output};
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub enum Step {
    Tap { keys: Vec<Key>, hold_ms: u64 },
    Delay(u64),
}
pub type Bindings = BTreeMap<Key, Vec<Step>>;
struct Run {
    steps: VecDeque<Step>,
    due: Instant,
    down: bool,
}
#[derive(Default)]
pub struct Player {
    runs: BTreeMap<Key, Run>,
}
fn owner(input: Key) -> Key {
    Key {
        scan: input.scan,
        extended: input.extended | 0x8000,
    }
}
impl Player {
    pub fn start(&mut self, input: Key, steps: &[Step], now: Instant) {
        self.runs.entry(input).or_insert_with(|| Run {
            steps: steps.to_vec().into(),
            due: now,
            down: false,
        });
    }
    pub fn tick(&mut self, engine: &mut Engine, now: Instant) -> Vec<Output> {
        let mut events = Vec::new();
        self.runs.retain(|input, run| {
            while run.due <= now {
                if run.down {
                    events.extend(engine.event(owner(*input), true));
                    run.down = false;
                }
                let Some(step) = run.steps.pop_front() else {
                    return false;
                };
                match step {
                    Step::Delay(ms) => run.due = now + Duration::from_millis(ms),
                    Step::Tap { keys, hold_ms } => {
                        events.extend(engine.virtual_press(owner(*input), keys));
                        run.down = true;
                        run.due = now + Duration::from_millis(hold_ms);
                    }
                }
            }
            true
        });
        events
    }
    pub fn wait_ms(&self, now: Instant) -> u32 {
        self.runs
            .values()
            .map(|r| {
                let d = r.due.saturating_duration_since(now);
                d.as_millis()
                    .saturating_add(u128::from(d.subsec_nanos() % 1_000_000 != 0))
                    .min(250) as u32
            })
            .min()
            .unwrap_or(250)
    }
    pub fn clear(&mut self) {
        self.runs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(s: u16) -> Key {
        Key {
            scan: s,
            extended: 0,
        }
    }
    fn tap(k: Key, ms: u64) -> Step {
        Step::Tap {
            keys: vec![k],
            hold_ms: ms,
        }
    }
    #[test]
    fn timing_delays_repeat_guard_and_no_catchup_burst() {
        let now = Instant::now();
        let mut p = Player::default();
        let mut e = Engine::default();
        let steps = [tap(key(30), 40), Step::Delay(100), tap(key(48), 40)];
        p.start(key(2), &steps, now);
        p.start(key(2), &steps, now);
        assert_eq!(
            p.tick(&mut e, now),
            vec![Output {
                key: key(30),
                up: false
            }]
        );
        assert_eq!(p.wait_ms(now), 40);
        assert!(p.tick(&mut e, now + Duration::from_millis(39)).is_empty());
        // Late wake releases A and starts a full pause, rather than emitting B immediately.
        assert_eq!(
            p.tick(&mut e, now + Duration::from_secs(1)),
            vec![Output {
                key: key(30),
                up: true
            }]
        );
        assert_eq!(p.wait_ms(now + Duration::from_secs(1)), 100);
        assert_eq!(
            p.tick(&mut e, now + Duration::from_millis(1100)),
            vec![Output {
                key: key(48),
                up: false
            }]
        );
        assert_eq!(
            p.tick(&mut e, now + Duration::from_millis(1140)),
            vec![Output {
                key: key(48),
                up: true
            }]
        );
        assert!(p.runs.is_empty());
        assert_eq!(e.owned_count(), 0);
        assert_eq!(p.wait_ms(now), 250);
    }
    #[test]
    fn concurrent_macros_and_ordinary_mapping_share_ownership_and_cancel() {
        let now = Instant::now();
        let shared = key(30);
        let mut e = Engine::new([(key(4), vec![shared])].into());
        let mut p = Player::default();
        e.event(key(4), false);
        for trigger in [key(2), key(3)] {
            p.start(trigger, &[tap(shared, 40)], now);
        }
        assert!(p.tick(&mut e, now).is_empty());
        assert!(p.tick(&mut e, now + Duration::from_millis(40)).is_empty());
        assert_eq!(
            e.event(key(4), true),
            vec![Output {
                key: shared,
                up: true
            }]
        );
        p.start(key(2), &[tap(shared, 500)], now);
        assert_eq!(p.tick(&mut e, now).len(), 1);
        p.clear();
        assert_eq!(
            e.replace(Default::default()),
            vec![Output {
                key: shared,
                up: true
            }]
        );
        // Synthetic owners must not be quarantined after cancellation.
        p.start(key(2), &[tap(shared, 40)], now);
        assert_eq!(p.tick(&mut e, now).len(), 1);
    }
}
