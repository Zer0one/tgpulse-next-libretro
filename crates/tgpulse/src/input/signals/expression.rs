//! Commas are alternatives; '&' is a simultaneous digital chord.
use crate::bindings::{parse_key, parse_source, Source};
use gilrs::Axis;
use winit::keyboard::KeyCode;

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    Source(Source),
    Axis(Axis, f32),
    Keys(KeyCode, KeyCode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub text: String,
    alternatives: Vec<Vec<Atom>>,
}
impl Binding {
    pub fn uses_source(&self, source: Source) -> bool {
        self.alternatives
            .iter()
            .flatten()
            .any(|atom| *atom == Atom::Source(source))
    }
    pub fn parse(text: &str, signed: bool) -> Result<Self, String> {
        let mut alternatives = Vec::new();
        if !text.trim().is_empty() {
            for alternative in text.split(',') {
                let mut chord = Vec::new();
                for token in alternative.split('&').map(str::trim) {
                    let atom = if let Some(pair) = token.strip_prefix("keys:") {
                        let (a, b) = pair
                            .split_once('/')
                            .ok_or("Expected keys:Negative/Positive")?;
                        Atom::Keys(
                            parse_key(a).ok_or("Unknown negative key")?,
                            parse_key(b).ok_or("Unknown positive key")?,
                        )
                    } else if let Some(source) = parse_source(token) {
                        Atom::Source(source)
                    } else if let Some(axis) = token.strip_prefix("pad:") {
                        let (name, sign) =
                            axis.strip_suffix('~').map_or((axis, 1.0), |a| (a, -1.0));
                        let axis = crate::bindings::PAD_AXES
                            .iter()
                            .find(|a| format!("{a:?}") == name)
                            .ok_or("Unknown axis")?;
                        Atom::Axis(*axis, sign)
                    } else {
                        return Err(format!("Unknown source: {token}"));
                    };
                    chord.push(atom);
                }
                if (!signed || chord.len() > 1)
                    && chord.iter().any(|a| !matches!(a, Atom::Source(_)))
                {
                    return Err(
                        "Full axes/key pairs require an axis signal and cannot appear in a chord"
                            .into(),
                    );
                }
                alternatives.push(chord);
            }
        }
        Ok(Self {
            text: text.trim().into(),
            alternatives,
        })
    }
    pub fn value(&self, mut read: impl FnMut(&Atom) -> f32) -> f32 {
        self.alternatives
            .iter()
            .map(|chord| {
                if chord.len() == 1 {
                    read(&chord[0])
                } else {
                    f32::from(u8::from(chord.iter().all(|a| read(a) > 0.5)))
                }
            })
            .fold(0.0, |best, value| {
                if value.abs() > best.abs() {
                    value
                } else {
                    best
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chords_require_all_terms_and_reject_partial_expressions() {
        let b = Binding::parse("KeyA & KeyB", false).unwrap();
        assert_eq!(
            b.value(|a| if *a == Atom::Source(Source::Key(KeyCode::KeyA)) {
                1.0
            } else {
                0.0
            }),
            0.0
        );
        assert_eq!(b.value(|_| 1.0), 1.0);
        assert_eq!(b.value(|_| 0.5), 0.0);
        for text in ["KeyA &", "KeyA, nonsense", "pad:LeftStickX & KeyA", ",KeyA"] {
            assert!(Binding::parse(text, true).is_err(), "{text}");
        }
    }
}
