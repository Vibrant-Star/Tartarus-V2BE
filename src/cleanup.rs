//! Release-only fallback for successfully delivered outputs after device loss.
use crate::mapping::{Key, Output};
#[repr(C)]
#[derive(Clone, Copy)]
struct Keyboard {
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Mouse {
    x: i32,
    y: i32,
    data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}
#[repr(C)]
union Data {
    keyboard: Keyboard,
    mouse: Mouse,
}
#[repr(C)]
struct Input {
    kind: u32,
    data: Data,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn SendInput(count: u32, input: *const Input, size: i32) -> u32;
}
fn release(key: Key) -> Result<Input, String> {
    let extra = 0x56324245;
    if let Some(packet) = crate::mouse::output_packet(Output { key, up: true })? {
        let (flags, data) = match packet.state {
            2 => (4, 0),
            8 => (0x10, 0),
            0x20 => (0x40, 0),
            0x80 => (0x100, 1),
            0x200 => (0x100, 2),
            _ => return Err("Invalid mouse release".into()),
        };
        return Ok(Input {
            kind: 0,
            data: Data {
                mouse: Mouse {
                    x: 0,
                    y: 0,
                    data,
                    flags,
                    time: 0,
                    extra,
                },
            },
        });
    }
    if !(1..=0x7f).contains(&key.scan) || !matches!(key.extended, 0 | 2) {
        return Err("Unsupported emergency keyboard release".into());
    }
    Ok(Input {
        kind: 1,
        data: Data {
            keyboard: Keyboard {
                vk: 0,
                scan: key.scan,
                flags: 8 | 2 | u32::from(key.extended == 2),
                time: 0,
                extra,
            },
        },
    })
}
pub fn release_owned(key: Key) -> Result<(), String> {
    let input = release(key)?;
    if unsafe { SendInput(1, &input, std::mem::size_of::<Input>() as i32) } != 1 {
        return Err("Windows refused release-only cleanup (possibly application privilege mismatch); recovery stopped".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_encodes_only_owned_key_and_button_releases() {
        assert_eq!(std::mem::size_of::<Input>(), 40);
        let i = release(Key {
            scan: 0x1d,
            extended: 2,
        })
        .unwrap();
        assert_eq!(i.kind, 1);
        let k = unsafe { i.data.keyboard };
        assert_eq!((k.vk, k.scan, k.flags), (0, 0x1d, 11));
        for (name, flags, data) in [
            ("left", 4, 0),
            ("right", 0x10, 0),
            ("middle", 0x40, 0),
            ("back", 0x100, 1),
            ("forward", 0x100, 2),
        ] {
            let i = release(crate::mouse::button(name).unwrap()).unwrap();
            assert_eq!(i.kind, 0);
            let m = unsafe { i.data.mouse };
            assert_eq!((m.flags, m.data, m.x, m.y), (flags, data, 0, 0));
        }
        assert!(release(crate::mouse::wheel("up").unwrap()).is_err());
        assert!(
            release(Key {
                scan: 0x100,
                extended: 0
            })
            .is_err()
        );
    }
}
