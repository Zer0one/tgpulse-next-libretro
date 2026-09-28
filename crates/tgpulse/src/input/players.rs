//! Frontend-only player/device assignment. No device handles enter the core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Player {
    #[default]
    One,
    Two,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pad(id: usize, kind: &str) -> (usize, String, String) {
        (id, kind.into(), "Controller".into())
    }
    #[test]
    fn auto_does_not_promote_player_two_on_disconnect_or_reorder() {
        let mut a = PadAssignments::default();
        let prefs = ["auto".into(), "auto".into()];
        a.observe(vec![pad(0, "same"), pad(1, "same")]);
        a.resolve(&prefs);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (Some(0), Some(1)));
        a.observe(vec![pad(1, "same")]);
        a.resolve(&prefs);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (None, Some(1)));
        a.observe(vec![pad(1, "same"), pad(0, "same")]);
        a.resolve(&prefs);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (Some(0), Some(1)));
        assert_eq!(a.devices.len(), 2);
    }
    #[test]
    fn explicit_devices_are_reserved_before_auto_and_duplicates_never_share() {
        let mut a = PadAssignments::default();
        a.observe(vec![pad(0, "a"), pad(1, "b")]);
        a.resolve(&["auto".into(), "a:1".into()]);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (Some(1), Some(0)));
        a.resolve(&["b:1".into(), "b:1".into()]);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (Some(1), None));
        a.resolve(&["none".into(), "auto".into()]);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (None, Some(0)));
    }
    #[test]
    fn persisted_selection_survives_id_changes_between_runs() {
        let mut a = PadAssignments::default();
        a.observe(vec![pad(10, "b"), pad(20, "a")]);
        a.resolve(&["a:1".into(), "b:1".into()]);
        assert_eq!((a.id(Player::One), a.id(Player::Two)), (Some(20), Some(10)));
        a.observe(vec![pad(10, "b")]);
        a.resolve(&["a:1".into(), "b:1".into()]);
        assert_eq!(a.id(Player::One), None);
        assert!(a.label(Player::One).contains("disconnected"));
    }
}
impl Player {
    pub const ALL: [Self; 2] = [Self::One, Self::Two];
    pub fn index(self) -> usize {
        if self == Self::One {
            0
        } else {
            1
        }
    }
}

#[derive(Clone, Debug)]
pub struct PadDevice {
    pub id: usize,
    pub key: String,
    pub label: String,
    pub connected: bool,
}

#[derive(Default)]
pub struct PadAssignments {
    pub devices: Vec<PadDevice>,
    selected: [Option<String>; 2],
    preferences: [String; 2],
}
impl PadAssignments {
    /// Retain disconnected identities, so losing P1 never promotes P2 to P1.
    /// UUID distinguishes models, ordinal distinguishes identical devices.
    /// Identical controllers may need re-selection after process restart.
    pub fn observe(&mut self, connected: Vec<(usize, String, String)>) {
        for d in &mut self.devices {
            d.connected = false;
        }
        for (id, uuid, name) in connected {
            if let Some(d) = self
                .devices
                .iter_mut()
                .find(|d| d.id == id && d.key.starts_with(&format!("{uuid}:")))
            {
                d.connected = true;
                continue;
            }
            let ordinal = self
                .devices
                .iter()
                .filter(|d| d.key.starts_with(&format!("{uuid}:")))
                .count()
                + 1;
            self.devices.push(PadDevice {
                id,
                key: format!("{uuid}:{ordinal}"),
                label: format!("{name} #{ordinal}"),
                connected: true,
            });
        }
    }

    pub fn resolve(&mut self, preferences: &[String; 2]) {
        for i in 0..2 {
            if self.preferences[i] != preferences[i] {
                self.selected[i] = None;
            }
        }
        self.preferences = preferences.clone();
        // Explicit assignments reserve their device before auto selection.
        for i in 0..2 {
            match preferences[i].as_str() {
                "auto" => {}
                "none" => self.selected[i] = None,
                key => self.selected[i] = Some(key.to_owned()),
            }
        }
        for i in 0..2 {
            let other = 1 - i;
            if preferences[i] == "auto" {
                if self.selected[i].is_some() && self.selected[i] == self.selected[other] {
                    self.selected[i] = None;
                }
                if self.selected[i].is_none() {
                    self.selected[i] = self
                        .devices
                        .iter()
                        .find(|d| d.connected && Some(&d.key) != self.selected[other].as_ref())
                        .map(|d| d.key.clone());
                }
            }
        }
        // Hand-edited duplicate explicit assignments cannot drive both players.
        if self.selected[0].is_some() && self.selected[0] == self.selected[1] {
            self.selected[1] = None;
        }
    }

    pub fn id(&self, player: Player) -> Option<usize> {
        let key = self.selected[player.index()].as_ref()?;
        self.devices
            .iter()
            .find(|d| &d.key == key && d.connected)
            .map(|d| d.id)
    }
    pub fn label(&self, player: Player) -> String {
        let Some(key) = &self.selected[player.index()] else {
            return "No controller".into();
        };
        self.devices
            .iter()
            .find(|d| &d.key == key)
            .map(|d| {
                format!(
                    "{}{}",
                    d.label,
                    if d.connected { "" } else { " (disconnected)" }
                )
            })
            .unwrap_or_else(|| format!("Disconnected: {key}"))
    }
}
