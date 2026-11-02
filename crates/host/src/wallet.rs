//! The player's account, for the phone's Nupay (owner 2026-11-03: "the account's balance and what came in and
//! went out"): its balance as the latest save says it (`save::money`), what the game moved since as it happened
//! (the plugin's money events: jobs, fines, tolls, ferries, trains), and their history, kept in the app's data.
//!
//! What the game doesn't tell as it happens (fuel, repairs, a truck or a garage bought, a loan's payment) shows at
//! the next save, as the difference between its balance and the one worked out: one entry, "other".

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use westplay_shared::{MONEY_EVENTS, MoneyEvents, MoneyKind};

/// The history's file, in the app's data.
const FILE: &str = "wallet.json";
/// How much history is kept, the oldest let go.
const MOST_ENTRIES: usize = 300;
/// How much of it the pages are sent.
const SENT_ENTRIES: usize = 100;

/// One sum in or out (an expense negative): what for (`delivered`, `fined`…, or `other`), about what, and when
/// (the PC's time, ms since 1970; the game's, minutes, when known).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: u64,
    pub game_time: Option<u32>,
    pub kind: String,
    pub amount: i64,
    pub detail: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct State {
    /// The balance the latest save had (`None`: no save read yet).
    saved: Option<i64>,
    /// What the game moved since that save.
    since_save: i64,
    /// Newest last.
    entries: Vec<Entry>,
}

pub struct Wallet {
    path: Option<PathBuf>,
    state: State,
    /// How many of the plugin's money events were taken (`None`: none looked at yet this run).
    seen: Option<u64>,
}

impl Wallet {
    /// The account as kept in `data_dir` (a new one where there's none): the game's `game` (its SDK id: each game's
    /// money its own, owner 2026-11-02, WestPlay in ETS2 as in ATS), ATS's in [`FILE`], there first.
    pub fn open(data_dir: &Path, game: &str) -> Self {
        let path = match westplay_shared::westplay_game_folder(game) {
            Some(folder) => data_dir.join(format!("wallet_{folder}.json")),
            None => data_dir.join(FILE),
        };
        let state = std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        Self { path: Some(path), state, seen: None }
    }

    /// The balance now: the latest save's, and what the game moved since.
    pub fn balance(&self) -> Option<i64> {
        self.state.saved.map(|saved| saved + self.state.since_save)
    }

    /// Takes the plugin's money events not taken yet; whether there were any. The ones already there when the app
    /// first looks were kept by the app that ran before (or happened while none did): not taken again.
    pub fn take(&mut self, events: &MoneyEvents) -> bool {
        let seen = match self.seen {
            None => {
                self.seen = Some(events.count);
                return false;
            }
            // The game started again: its count did too.
            Some(seen) if events.count < seen => 0,
            Some(seen) => seen,
        };
        if events.count == seen {
            return false;
        }
        let first = seen.max(events.count.saturating_sub(MONEY_EVENTS as u64));
        for n in first..events.count {
            let event = &events.events[(n % MONEY_EVENTS as u64) as usize];
            let Some(kind) = MoneyKind::from_code(event.kind) else { continue };
            self.state.since_save += event.amount;
            self.push(Entry {
                at: now(),
                game_time: Some(event.game_time),
                kind: kind.name().to_owned(),
                amount: event.amount,
                detail: event.detail.as_str().to_owned(),
            });
        }
        self.seen = Some(events.count);
        self.keep();
        true
    }

    /// A save was read, its balance `money`: what it says that wasn't worked out is entered as `other`. Whether
    /// anything changed.
    pub fn saved(&mut self, money: i64) -> bool {
        let expected = self.balance();
        if expected == Some(money) && self.state.since_save == 0 {
            return false;
        }
        if let Some(expected) = expected
            && money != expected
        {
            self.push(Entry {
                at: now(),
                game_time: None,
                kind: "other".into(),
                amount: money - expected,
                detail: String::new(),
            });
        }
        self.state.saved = Some(money);
        self.state.since_save = 0;
        self.keep();
        true
    }

    /// `{"type":"wallet","balance":25512,"entries":[…]}`: the account for the pages, the newest entries first.
    pub fn message(&self) -> String {
        let entries: Vec<&Entry> = self.state.entries.iter().rev().take(SENT_ENTRIES).collect();
        serde_json::json!({ "type": "wallet", "balance": self.balance(), "entries": entries }).to_string()
    }

    fn push(&mut self, entry: Entry) {
        self.state.entries.push(entry);
        let over = self.state.entries.len().saturating_sub(MOST_ENTRIES);
        self.state.entries.drain(..over);
    }

    /// Writes the account down (logged, not fatal, if it can't be).
    fn keep(&self) {
        let Some(path) = &self.path else { return };
        let written = serde_json::to_vec(&self.state).map_err(|e| e.to_string()).and_then(|bytes| {
            let partial = path.with_extension("json.tmp");
            std::fs::write(&partial, bytes).and_then(|()| std::fs::rename(&partial, path)).map_err(|e| e.to_string())
        });
        if let Err(e) = written {
            log!("wallet: couldn't keep it: {e}");
        }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use westplay_shared::{FixedStr, MoneyEvent};

    use super::*;

    fn wallet() -> Wallet {
        Wallet { path: None, state: State::default(), seen: None }
    }

    fn with(events: &[(MoneyKind, i64)]) -> MoneyEvents {
        let mut all = MoneyEvents::default();
        for (kind, amount) in events {
            all.events[(all.count % MONEY_EVENTS as u64) as usize] =
                MoneyEvent { kind: *kind as u32, game_time: 600, amount: *amount, detail: FixedStr::new("x") };
            all.count += 1;
        }
        all
    }

    #[test]
    fn the_balance_is_the_saves_and_what_the_game_moved_since() {
        let mut wallet = wallet();
        assert_eq!(wallet.balance(), None);
        assert!(wallet.saved(25_000));
        // Events there when the app first looks were the app before's.
        assert!(!wallet.take(&with(&[(MoneyKind::Toll, -10)])));
        assert!(wallet.take(&with(&[
            (MoneyKind::Toll, -10),
            (MoneyKind::Delivered, 21_469),
            (MoneyKind::Fined, -300)
        ])));
        assert_eq!(wallet.balance(), Some(25_000 + 21_469 - 300));
        assert_eq!(wallet.state.entries.len(), 2);
    }

    #[test]
    fn what_a_save_says_unexplained_is_other() {
        let mut wallet = wallet();
        wallet.saved(25_000);
        wallet.take(&with(&[]));
        wallet.take(&with(&[(MoneyKind::Delivered, 1_000)]));
        // Fuel bought for 400: the next save says 25 600.
        assert!(wallet.saved(25_600));
        let other = wallet.state.entries.last().unwrap();
        assert_eq!((other.kind.as_str(), other.amount), ("other", -400));
        assert_eq!(wallet.balance(), Some(25_600));
        // The same save again: nothing new.
        assert!(!wallet.saved(25_600));
    }

    #[test]
    fn a_game_started_again_counts_from_nothing() {
        let mut wallet = wallet();
        wallet.saved(0);
        wallet.take(&with(&[(MoneyKind::Toll, -5), (MoneyKind::Toll, -5)]));
        assert!(wallet.take(&with(&[(MoneyKind::Ferry, -150)])));
        assert_eq!(wallet.balance(), Some(-150));
    }
}
