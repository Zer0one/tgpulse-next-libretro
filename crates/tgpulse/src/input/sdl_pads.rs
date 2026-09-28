//! SDL3 gamepads behind the desktop input boundary. Only logical controls
//! cross into the existing signal bindings; no SDL handle enters the core.
use std::collections::BTreeMap;

use gilrs::{Axis, Button};
use sdl3::gamepad::{Axis as SdlAxis, Button as SdlButton, Gamepad};
use sdl3::{EventPump, GamepadSubsystem};

pub struct SdlPads {
    // Keep the context alive for the subsystem, event pump and open handles.
    _context: sdl3::Sdl,
    subsystem: GamepadSubsystem,
    events: EventPump,
    pads: BTreeMap<usize, Gamepad>,
}

impl SdlPads {
    pub fn new() -> Result<Self, String> {
        let context = sdl3::init().map_err(|e| e.to_string())?;
        let subsystem = context.gamepad().map_err(|e| e.to_string())?;
        let events = context.event_pump().map_err(|e| e.to_string())?;
        let mut this = Self {
            _context: context,
            subsystem,
            events,
            pads: BTreeMap::new(),
        };
        this.poll();
        Ok(this)
    }

    pub fn poll(&mut self) {
        // SDL updates gamepad state while pumping events. Enumeration, rather
        // than queued add/remove events alone, also catches startup devices.
        for _ in self.events.poll_iter() {}
        match self.subsystem.gamepads() {
            Ok(ids) => {
                self.pads
                    .retain(|id, _| ids.iter().any(|s| s.raw() as usize == *id));
                for id in ids {
                    let key = id.raw() as usize;
                    if !self.pads.contains_key(&key) {
                        match self.subsystem.open(id) {
                            Ok(pad) => {
                                self.pads.insert(key, pad);
                            }
                            Err(e) => {
                                log::warn!(target: "input", "SDL3 cannot open gamepad {key}: {e}")
                            }
                        }
                    }
                }
            }
            Err(e) => log::warn!(target: "input", "SDL3 cannot enumerate gamepads: {e}"),
        }
    }

    pub fn devices(&self) -> Vec<(usize, String, String)> {
        self.pads
            .iter()
            .map(|(&id, pad)| {
                let guid = self
                    .subsystem
                    .guid_for_id(sdl3::joystick::JoystickId::new(id as u32));
                (
                    id,
                    guid.string(),
                    pad.name().unwrap_or_else(|| "Gamepad".into()),
                )
            })
            .collect()
    }

    pub fn has(&self, id: usize) -> bool {
        self.pads.contains_key(&id)
    }

    pub fn button(&self, id: usize, button: Button) -> bool {
        let Some(pad) = self.pads.get(&id) else {
            return false;
        };
        if let Some(mapped) = sdl_button(button) {
            return pad.button(mapped);
        }
        match button {
            Button::LeftTrigger2 => pad.axis(SdlAxis::TriggerLeft) > 1_638,
            Button::RightTrigger2 => pad.axis(SdlAxis::TriggerRight) > 1_638,
            _ => false,
        }
    }

    pub fn axis(&self, id: usize, axis: Axis) -> f32 {
        let Some(pad) = self.pads.get(&id) else {
            return 0.0;
        };
        if axis == Axis::DPadX {
            return f32::from(u8::from(pad.button(SdlButton::DPadRight)))
                - f32::from(u8::from(pad.button(SdlButton::DPadLeft)));
        }
        if axis == Axis::DPadY {
            return f32::from(u8::from(pad.button(SdlButton::DPadUp)))
                - f32::from(u8::from(pad.button(SdlButton::DPadDown)));
        }
        let Some(mapped) = sdl_axis(axis) else {
            return 0.0;
        };
        normalize_axis(axis, pad.axis(mapped))
    }

    pub fn rumble(&mut self, id: usize, low: f32, high: f32) {
        if let Some(pad) = self.pads.get_mut(&id) {
            let low = (low.clamp(0.0, 1.0) * u16::MAX as f32) as u16;
            let high = (high.clamp(0.0, 1.0) * u16::MAX as f32) as u16;
            // Commands are refreshed during gameplay; a short duration ensures
            // force cannot remain latched after pause, disconnect or exit.
            let _ = pad.set_rumble(low, high, if low == 0 && high == 0 { 0 } else { 100 });
        }
    }
}

impl Drop for SdlPads {
    fn drop(&mut self) {
        for pad in self.pads.values_mut() {
            let _ = pad.set_rumble(0, 0, 0);
        }
    }
}

fn sdl_button(button: Button) -> Option<SdlButton> {
    Some(match button {
        Button::South => SdlButton::South,
        Button::East => SdlButton::East,
        Button::North => SdlButton::North,
        Button::West => SdlButton::West,
        Button::LeftTrigger => SdlButton::LeftShoulder,
        Button::RightTrigger => SdlButton::RightShoulder,
        Button::Select => SdlButton::Back,
        Button::Start => SdlButton::Start,
        Button::Mode => SdlButton::Guide,
        Button::LeftThumb => SdlButton::LeftStick,
        Button::RightThumb => SdlButton::RightStick,
        Button::DPadUp => SdlButton::DPadUp,
        Button::DPadDown => SdlButton::DPadDown,
        Button::DPadLeft => SdlButton::DPadLeft,
        Button::DPadRight => SdlButton::DPadRight,
        _ => return None,
    })
}

fn sdl_axis(axis: Axis) -> Option<SdlAxis> {
    Some(match axis {
        Axis::LeftStickX => SdlAxis::LeftX,
        Axis::LeftStickY => SdlAxis::LeftY,
        Axis::RightStickX => SdlAxis::RightX,
        Axis::RightStickY => SdlAxis::RightY,
        Axis::LeftZ => SdlAxis::TriggerLeft,
        Axis::RightZ => SdlAxis::TriggerRight,
        _ => return None,
    })
}

fn normalize_axis(axis: Axis, raw: i16) -> f32 {
    let value = if matches!(axis, Axis::LeftZ | Axis::RightZ) {
        raw.max(0) as f32 / i16::MAX as f32
    } else {
        raw as f32 / if raw < 0 { 32768.0 } else { 32767.0 }
    };
    if matches!(axis, Axis::LeftStickY | Axis::RightStickY) {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdl_axes_match_existing_signed_convention() {
        assert_eq!(normalize_axis(Axis::LeftStickY, i16::MIN), 1.0);
        assert_eq!(normalize_axis(Axis::RightStickY, i16::MAX), -1.0);
        assert_eq!(normalize_axis(Axis::LeftZ, 0), 0.0);
        assert_eq!(normalize_axis(Axis::RightZ, i16::MAX), 1.0);
    }
    #[test]
    fn logical_buttons_do_not_alias_dpad_and_stick_click() {
        assert_eq!(sdl_button(Button::LeftThumb), Some(SdlButton::LeftStick));
        assert_eq!(sdl_button(Button::RightThumb), Some(SdlButton::RightStick));
        assert_ne!(
            sdl_button(Button::LeftThumb),
            sdl_button(Button::RightThumb)
        );
        assert_eq!(sdl_button(Button::DPadRight), Some(SdlButton::DPadRight));
    }
}
