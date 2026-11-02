//! The truck's commands the phones hold (the phone in the game's Commands app): which are held now, for the
//! plugin's input device (plugin `commands.rs`, through `HostBlock::truck_commands`).
//!
//! A press is held at least [`SHORTEST`], so a tap (down and up at once) still reaches the game, which reads its
//! inputs once a frame; and at most [`LONGEST`], so a phone gone while holding one (its page closed, its Wi-Fi
//! lost) doesn't hold it for ever.

use std::time::{Duration, Instant};

use westplay_shared::{TRUCK_COMMANDS, truck_command_bit};

/// How long a press is held at the least: a few of the game's frames, even at a low frame rate.
const SHORTEST: Duration = Duration::from_millis(150);
/// How long a press is held at the most: a window takes a few seconds to go all the way.
const LONGEST: Duration = Duration::from_secs(10);

pub struct TruckCommands {
    /// Since when each command (by its bit's index) is held.
    since: [Option<Instant>; 64],
    /// The commands let go before they were held [`SHORTEST`]: let go once they have been.
    letting_go: u64,
}

impl Default for TruckCommands {
    fn default() -> Self {
        Self { since: [None; 64], letting_go: 0 }
    }
}

impl TruckCommands {
    /// A phone pressed (`down`) or let go of the command `name` (unknown ones are ignored).
    pub fn press(&mut self, name: &str, down: bool, now: Instant) {
        let Some(bit) = truck_command_bit(name) else { return };
        let index = bit.trailing_zeros() as usize;
        if down {
            self.since[index] = Some(now);
            self.letting_go &= !bit;
        } else {
            self.letting_go |= bit;
        }
    }

    /// The commands held now, bit by bit, the ones let go and done with forgotten.
    pub fn held(&mut self, now: Instant) -> u64 {
        let mut held = 0;
        for index in 0..TRUCK_COMMANDS.len() {
            let Some(since) = self.since[index] else { continue };
            let bit = 1u64 << index;
            let long = now.saturating_duration_since(since);
            if (self.letting_go & bit != 0 && long >= SHORTEST) || long >= LONGEST {
                self.since[index] = None;
                self.letting_go &= !bit;
            } else {
                held |= bit;
            }
        }
        held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tap_is_held_long_enough_for_the_game() {
        let start = Instant::now();
        let mut commands = TruckCommands::default();
        commands.press("beacon", true, start);
        commands.press("beacon", false, start);
        let bit = truck_command_bit("beacon").unwrap();
        assert_eq!(commands.held(start + Duration::from_millis(50)), bit);
        assert_eq!(commands.held(start + SHORTEST), 0);
    }

    #[test]
    fn a_press_lasts_until_let_go_but_not_for_ever() {
        let start = Instant::now();
        let mut commands = TruckCommands::default();
        commands.press("lwinopen", true, start);
        let bit = truck_command_bit("lwinopen").unwrap();
        assert_eq!(commands.held(start + Duration::from_secs(3)), bit);
        assert_eq!(commands.held(start + LONGEST), 0);
    }

    #[test]
    fn unknown_commands_are_ignored() {
        let mut commands = TruckCommands::default();
        commands.press("quit", true, Instant::now());
        assert_eq!(commands.held(Instant::now()), 0);
    }
}
