//! Pure splitting of combined native mouse reports into mapped controls/residuals.
use crate::mapping::{Bindings, Key};
// Internal output namespace; never passed to the keyboard driver as scan codes.
const OUTPUT: u16 = 0x8000;
pub fn button(name: &str) -> Result<Key, String> {
    let scan = match name {
        "left" => 1,
        "right" => 2,
        "middle" => 3,
        "back" => 4,
        "forward" => 5,
        _ => return Err(format!("Unsupported mouse button {name:?}")),
    };
    Ok(Key {
        scan,
        extended: OUTPUT,
    })
}
pub fn wheel(name: &str) -> Result<Key, String> {
    let scan = match name {
        "up" => 6,
        "down" => 7,
        "left" => 8,
        "right" => 9,
        _ => return Err(format!("Unsupported wheel direction {name:?}")),
    };
    Ok(Key {
        scan,
        extended: OUTPUT,
    })
}
pub fn is_button(key: Key) -> bool {
    key.extended == OUTPUT && (1..=5).contains(&key.scan)
}
pub fn is_wheel(key: Key) -> bool {
    key.extended == OUTPUT && (6..=9).contains(&key.scan)
}
pub fn label(key: Key) -> Option<&'static str> {
    if key.extended != OUTPUT {
        return None;
    }
    Some(match key.scan {
        1 => "Mouse left",
        2 => "Mouse right",
        3 => "Mouse middle",
        4 => "Mouse back",
        5 => "Mouse forward",
        6 => "Wheel up",
        7 => "Wheel down",
        8 => "Wheel left",
        9 => "Wheel right",
        _ => return None,
    })
}
pub fn output_packet(output: crate::mapping::Output) -> Result<Option<MouseStroke>, String> {
    if output.key.extended != OUTPUT {
        return Ok(None);
    }
    let mut packet = MouseStroke::default();
    if is_button(output.key) {
        packet.state = (1 << ((output.key.scan - 1) * 2)) << u16::from(output.up);
    } else if is_wheel(output.key) {
        if output.up {
            return Err("Wheel output has no release event".into());
        }
        packet.state = if output.key.scan <= 7 { 0x400 } else { 0x800 };
        packet.rolling = if matches!(output.key.scan, 6 | 9) {
            120
        } else {
            -120
        };
    } else {
        return Err("Invalid internal mouse output".into());
    }
    Ok(Some(packet))
}
pub const WHEEL_UP: Key = Key {
    scan: 0x100,
    extended: 0,
};
pub const WHEEL_DOWN: Key = Key {
    scan: 0x101,
    extended: 0,
};
pub const MIDDLE: Key = Key {
    scan: 0x102,
    extended: 0,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseStroke {
    pub state: u16,
    pub flags: u16,
    pub rolling: i16,
    pub x: i32,
    pub y: i32,
    pub information: u32,
}
impl MouseStroke {
    pub fn has_event(&self) -> bool {
        self.state != 0 || self.flags != 0 || self.x != 0 || self.y != 0
    }
}
#[derive(Default)]
pub struct MouseMapper {
    remainder: i32,
    middle_route: Option<bool>, // decision at press survives config changes
}
impl MouseMapper {
    pub fn reset_wheel(&mut self) {
        self.remainder = 0;
    }
    pub fn split(
        &mut self,
        mut packet: MouseStroke,
        bindings: &Bindings,
    ) -> (MouseStroke, Vec<(Key, bool)>) {
        let mut events = vec![];
        for (bit, up) in [(0x10, false), (0x20, true)] {
            if packet.state & bit != 0 {
                let mapped = if up {
                    self.middle_route.take().unwrap_or(false)
                } else {
                    let mapped = bindings.contains_key(&MIDDLE);
                    self.middle_route = Some(mapped);
                    mapped
                };
                if mapped {
                    packet.state &= !bit;
                    events.push((MIDDLE, up));
                }
            }
        }
        if packet.state & 0x400 != 0 && packet.rolling != 0 {
            let input = if packet.rolling > 0 {
                WHEEL_UP
            } else {
                WHEEL_DOWN
            };
            if bindings.contains_key(&input) {
                self.remainder += i32::from(packet.rolling);
                packet.state &= !0x400;
                // The API shares one delta field with horizontal scrolling.
                if packet.state & 0x800 == 0 {
                    packet.rolling = 0;
                }
                while self.remainder.abs() >= 120 {
                    let key = if self.remainder > 0 {
                        WHEEL_UP
                    } else {
                        WHEEL_DOWN
                    };
                    self.remainder -= if self.remainder > 0 { 120 } else { -120 };
                    events.push((key, false));
                    events.push((key, true));
                }
            } else {
                self.remainder = 0;
            }
        }
        (packet, events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_packets_are_isolated_from_input_ids_and_have_no_motion() {
        use crate::mapping::Output;
        for (name, bit) in [
            ("left", 1),
            ("right", 4),
            ("middle", 16),
            ("back", 64),
            ("forward", 256),
        ] {
            let key = button(name).unwrap();
            assert!(![WHEEL_UP, WHEEL_DOWN, MIDDLE].contains(&key));
            for up in [false, true] {
                let p = output_packet(Output { key, up }).unwrap().unwrap();
                assert_eq!(
                    p,
                    MouseStroke {
                        state: if up { bit << 1 } else { bit },
                        ..Default::default()
                    }
                );
            }
        }
        for (name, state, rolling) in [
            ("up", 0x400, 120),
            ("down", 0x400, -120),
            ("left", 0x800, -120),
            ("right", 0x800, 120),
        ] {
            let key = wheel(name).unwrap();
            assert_eq!(
                output_packet(Output { key, up: false }).unwrap().unwrap(),
                MouseStroke {
                    state,
                    rolling,
                    ..Default::default()
                }
            );
            assert!(output_packet(Output { key, up: true }).is_err());
        }
        assert!(button("typo").is_err());
        assert!(wheel("diagonal").is_err());
        assert!(
            output_packet(Output {
                key: Key {
                    scan: 99,
                    extended: OUTPUT
                },
                up: false
            })
            .is_err()
        );
    }
    fn bindings() -> Bindings {
        [(WHEEL_UP, vec![]), (WHEEL_DOWN, vec![]), (MIDDLE, vec![])].into()
    }
    #[test]
    fn layout_and_combined_report_preserve_unmapped_fields() {
        assert_eq!(std::mem::size_of::<MouseStroke>(), 20);
        let p = MouseStroke {
            state: 0x411,
            rolling: 120,
            x: 7,
            y: -4,
            information: 123,
            ..Default::default()
        };
        let (residual, events) = MouseMapper::default().split(p, &bindings());
        assert_eq!(residual.state, 1); // left button preserved
        assert_eq!((residual.x, residual.y, residual.information), (7, -4, 123));
        assert_eq!(
            events,
            vec![(MIDDLE, false), (WHEEL_UP, false), (WHEEL_UP, true)]
        );
    }
    #[test]
    fn multiple_notches_partial_deltas_and_reversal() {
        let mut m = MouseMapper::default();
        let p = |delta| MouseStroke {
            state: 0x400,
            rolling: delta,
            ..Default::default()
        };
        assert!(m.split(p(60), &bindings()).1.is_empty());
        assert_eq!(m.split(p(60), &bindings()).1.len(), 2);
        assert_eq!(m.split(p(-360), &bindings()).1.len(), 6);
        m.split(p(60), &bindings());
        assert!(m.split(p(-60), &bindings()).1.is_empty());
    }
    #[test]
    fn middle_route_survives_config_change_both_ways() {
        let mut m = MouseMapper::default();
        let down = MouseStroke {
            state: 0x10,
            ..Default::default()
        };
        let up = MouseStroke {
            state: 0x20,
            ..Default::default()
        };
        m.split(down, &bindings());
        assert_eq!(m.split(up, &Bindings::new()).1, vec![(MIDDLE, true)]);
        assert_eq!(m.split(down, &Bindings::new()).0, down);
        let (native, events) = m.split(up, &bindings());
        assert_eq!(native, up);
        assert!(events.is_empty());
    }
    #[test]
    fn unmapped_reports_pass_exactly_and_horizontal_scroll_is_preserved() {
        let p = MouseStroke {
            state: 0x810,
            rolling: -120,
            flags: 1,
            x: 42,
            information: 5,
            ..Default::default()
        };
        assert_eq!(
            MouseMapper::default().split(p, &Bindings::new()),
            (p, vec![])
        );
        let (residual, _) = MouseMapper::default().split(p, &bindings());
        assert_eq!(residual.state, 0x800);
        assert_eq!(residual.rolling, -120);
        let both = MouseStroke {
            state: 0xc00,
            rolling: 120,
            ..Default::default()
        };
        let (residual, events) = MouseMapper::default().split(both, &bindings());
        assert_eq!(residual.state, 0x800);
        assert_eq!(residual.rolling, 120);
        assert_eq!(events, vec![(WHEEL_UP, false), (WHEEL_UP, true)]);
    }
}
