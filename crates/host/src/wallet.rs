//! The player's account, for the phone's Nupay (owner 2026-11-03 / 2026-11-24):
//! - When the game plugin is live, mirrors the exact in-game `bank_u->money_account`, active loans
//!   (with exact daily installments, paid/remaining days, and instant repayment), and live in-game
//!   `bank_data_u->loan_offers` directly from memory without restarting the game.
//! - Falls back gracefully to the latest save (`game.sii`) when the game is not running.
//! - Keeps transaction history (deliveries, fines, tolls, ferries, trains, fuel, loans, repayments).

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use westplay_shared::{
    BankAction, BankAnswer, BankBlock, LIVE_LOANS_MAX, LIVE_OFFERS_MAX, LiveBank, MONEY_EVENTS, MoneyEvents,
    MoneyKind,
};

/// The history's file, in the app's data.
const FILE: &str = "wallet.json";
/// How much history is kept, the oldest let go.
const MOST_ENTRIES: usize = 300;
/// How much of it the pages are sent.
const SENT_ENTRIES: usize = 100;
/// Timeout for in-memory Bank RPCs.
const BANK_RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// One sum in or out (an expense negative): what for (`delivered`, `fined`, `loan`, `loanRepay`…, or `other`),
/// about what, and when (the PC's time, ms since 1970; the game's, minutes, when known).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: u64,
    pub game_time: Option<u32>,
    pub kind: String,
    pub amount: i64,
    pub detail: String,
}

/// Active bank loan (from live `bank_u->loans` or `game.sii`), shown in Nupay's Financiamentos tab.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoanInfo {
    pub index: usize,
    pub amount: i64,
    pub original_amount: i64,
    pub paid_amount: i64,
    pub interest_rate: f32,
    /// Duration in days (`duration_weeks * 7`).
    pub duration: i32,
    pub duration_weeks: u32,
    pub daily_payment: i64,
    pub daily_principal: i64,
    pub days_total: u32,
    pub days_paid: u32,
    pub days_remaining: u32,
    pub total_repayment: i64,
    pub can_repay: bool,
}

/// Live in-game bank loan offer read directly from `bank_data_u->loan_offers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoanOfferInfo {
    pub index: usize,
    pub amount: i64,
    pub interest_rate: f32,
    pub duration_weeks: u32,
    pub duration_days: u32,
    pub daily_payment: i64,
    pub total_repayment: i64,
    pub can_take: bool,
}

/// Legacy queued loan (kept for offline/save fallback when the game isn't running).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLoan {
    pub amount: i64,
    pub days: i32,
    pub rate: f32,
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
    /// Pending loan requested when the game wasn't running.
    pending_loan: Option<PendingLoan>,
}

pub struct Wallet {
    path: Option<PathBuf>,
    state: State,
    /// Exact live in-game balance from `bank_u->money_account` (when connected to live plugin).
    live_balance: Option<i64>,
    /// Whether live in-memory bank mirror is active right now.
    live_bank: bool,
    /// Whether the player has unlocked the in-game bank (`bank_u::app_enabled`).
    bank_unlocked: bool,
    /// Exact live maximum loan limit (`bank_u::loan_limit`).
    loan_limit: i64,
    /// Exact live available credit (`loan_limit - total_owed`).
    available_credit: i64,
    /// Minimum balance kept after repaying a loan (`bank_data_u::min_after_loan_repay`).
    min_after_repay: i32,
    /// Active bank loans (preferring live memory, falling back to latest save).
    loans: Vec<LoanInfo>,
    /// Live in-game loan offers from `bank_data_u->loan_offers`.
    offers: Vec<LoanOfferInfo>,
    /// Latest `game.sii` path seen by `PlayerWatch`, for offline fallback.
    save_path: Option<PathBuf>,
    /// How many of the plugin's money events were taken (`None`: none looked at yet this run).
    seen: Option<u64>,
    /// Last `BankBlock::live_seq` observed from the plugin.
    last_live_seq: u32,
    /// The currency the game counts its money in (ISO 4217): ETS2's euros, ATS's dollars.
    currency: &'static str,
    /// The entries made since the last `take_fresh`, for the phone's notifications (alerts.rs).
    fresh: Vec<Entry>,
    /// Whether a save was read this run.
    save_read: bool,
}

impl Wallet {
    /// The account as kept in `data_dir` (a new one where there's none): the game's `game` (its SDK id).
    pub fn open(data_dir: &Path, game: &str) -> Self {
        let path = match westplay_shared::westplay_game_folder(game) {
            Some(folder) => data_dir.join(format!("wallet_{folder}.json")),
            None => data_dir.join(FILE),
        };
        let state = std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        Self {
            path: Some(path),
            state,
            live_balance: None,
            live_bank: false,
            bank_unlocked: true,
            loan_limit: 0,
            available_credit: 0,
            min_after_repay: 0,
            loans: Vec::new(),
            offers: Vec::new(),
            save_path: None,
            seen: None,
            last_live_seq: 0,
            currency: currency_of(game),
            fresh: Vec::new(),
            save_read: false,
        }
    }

    /// The exact balance now: live `bank_u->money_account` when the game is running, else latest save + delta.
    pub fn balance(&self) -> Option<i64> {
        self.live_balance.or_else(|| self.state.saved.map(|saved| saved + self.state.since_save))
    }

    /// The currency the game counts its money in (ISO 4217).
    pub fn currency(&self) -> &'static str {
        self.currency
    }

    /// Resets live-memory connection state when the game closes.
    pub fn on_game_stopped(&mut self) -> bool {
        if !self.live_bank {
            return false;
        }
        if let Some(live) = self.live_balance.take() {
            self.state.saved = Some(live);
            self.state.since_save = 0;
            self.keep();
        }
        self.live_bank = false;
        self.last_live_seq = 0;
        true
    }

    /// Polls `BankBlock::live` from shared memory when `live_seq` advances; returns `true` if Nupay state changed.
    pub fn poll_live_bank(&mut self, block: &BankBlock) -> bool {
        let seq = block.live_seq.load(Ordering::Acquire);
        if seq == 0 || seq == self.last_live_seq {
            return false;
        }
        let Some(live) = block.live.read() else { return false };
        self.last_live_seq = seq;
        self.sync_live_bank(&live)
    }

    /// Synchronizes Nupay with a live `LiveBank` snapshot read directly from `economy_u->bank`.
    pub fn sync_live_bank(&mut self, live: &LiveBank) -> bool {
        if live.valid == 0 {
            return false;
        }
        let prev_live = self.live_balance;
        let prev_loans = self.loans.clone();
        let prev_offers = self.offers.clone();
        let prev_meta = (self.live_bank, self.bank_unlocked, self.loan_limit, self.available_credit, self.min_after_repay);

        self.live_bank = true;
        self.live_balance = Some(live.balance);
        self.bank_unlocked = live.app_enabled != 0;
        self.loan_limit = live.loan_limit;
        self.available_credit = live.available_credit;
        self.min_after_repay = live.min_after_repay;
        // Keep persisted baseline aligned with live balance so restart/offline view never drifts.
        self.state.saved = Some(live.balance);
        self.state.since_save = 0;

        let loans_len = (live.loans_count as usize).min(LIVE_LOANS_MAX);
        let mut next_loans = Vec::with_capacity(loans_len);
        for (idx, l) in live.loans[..loans_len].iter().enumerate() {
            let paid_amount = l.original_amount.saturating_sub(l.amount).max(0);
            let total_repayment = l.daily_payment.saturating_mul(i64::from(l.days_total));
            next_loans.push(LoanInfo {
                index: idx,
                amount: l.amount,
                original_amount: l.original_amount,
                paid_amount,
                interest_rate: l.interest_rate,
                duration: l.days_total as i32,
                duration_weeks: l.duration_weeks,
                daily_payment: l.daily_payment,
                daily_principal: l.daily_principal,
                days_total: l.days_total,
                days_paid: l.days_paid,
                days_remaining: l.days_remaining,
                total_repayment,
                can_repay: l.can_repay != 0,
            });
        }
        self.loans = next_loans;

        let offers_len = (live.offers_count as usize).min(LIVE_OFFERS_MAX);
        let mut next_offers = Vec::with_capacity(offers_len);
        for o in &live.offers[..offers_len] {
            next_offers.push(LoanOfferInfo {
                index: o.index as usize,
                amount: o.amount,
                interest_rate: o.interest_rate,
                duration_weeks: o.duration_weeks,
                duration_days: o.duration_weeks.saturating_mul(7),
                daily_payment: o.daily_payment,
                total_repayment: o.total_repayment,
                can_take: o.can_take != 0,
            });
        }
        self.offers = next_offers;

        let changed = prev_live != self.live_balance
            || prev_loans != self.loans
            || prev_offers != self.offers
            || prev_meta != (self.live_bank, self.bank_unlocked, self.loan_limit, self.available_credit, self.min_after_repay);
        if changed && prev_live != self.live_balance {
            self.keep();
        }
        changed
    }

    /// Records a loan taken or repaid directly in Nupay's history.
    pub fn record_bank_entry(&mut self, kind: &str, amount: i64, detail: &str) {
        self.push(Entry { at: now(), game_time: None, kind: kind.to_owned(), amount, detail: detail.to_owned() });
        self.keep();
    }

    /// Looks up a live offer's amount & days by `offer_index` (for clean Extrato detail after `TakeLoan`).
    pub fn offer_summary(&self, offer_index: usize) -> Option<(i64, u32, f32)> {
        self.offers
            .iter()
            .find(|o| o.index == offer_index)
            .map(|o| (o.amount, o.duration_days, o.interest_rate))
    }

    /// Looks up an active loan's remaining amount & original amount by `loan_index` (for Extrato after `RepayLoan`).
    pub fn loan_summary(&self, loan_index: usize) -> Option<(i64, i64)> {
        self.loans.iter().find(|l| l.index == loan_index).map(|l| (l.amount, l.original_amount))
    }

    /// The entries made since this was last asked (the phone's notifications of money in and out).
    pub fn take_fresh(&mut self) -> Vec<Entry> {
        std::mem::take(&mut self.fresh)
    }

    /// Takes the plugin's money events not taken yet; whether there were any.
    pub fn take(&mut self, events: &MoneyEvents) -> bool {
        let seen = match self.seen {
            None => {
                self.seen = Some(events.count);
                return false;
            }
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
            if !self.live_bank {
                self.state.since_save += event.amount;
            }
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

    /// A save was read, its balance `money`. When live memory is active, the live `bank_u->money_account`
    /// remains authoritative so a stale save file can never overwrite the real balance!
    pub fn saved(&mut self, money: i64) -> bool {
        if self.live_bank {
            self.save_read = true;
            return false;
        }
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
            if !self.save_read {
                self.fresh.clear();
            }
        }
        self.save_read = true;
        self.state.saved = Some(money);
        self.state.since_save = 0;
        self.keep();
        true
    }

    /// A sum paid on the phone itself (`kind`: `fuel`, the pump's bill paid at its attendant).
    pub fn paid(&mut self, kind: &str, amount: i64, detail: &str) {
        if !self.live_bank {
            self.state.since_save += amount;
        }
        self.push(Entry { at: now(), game_time: None, kind: kind.to_owned(), amount, detail: detail.to_owned() });
        self.keep();
    }

    /// Lets the history go, the balance kept.
    pub fn clear_history(&mut self) {
        self.state.entries.clear();
        self.keep();
    }

    /// Updates active loans and save path from the latest save (only used when live bank isn't connected).
    pub fn update_save_loans(&mut self, loans: Vec<westplay_mapdata::save::Loan>, save_path: Option<PathBuf>) -> bool {
        if save_path.is_some() {
            self.save_path = save_path;
        }
        if self.live_bank {
            return false;
        }
        let next: Vec<LoanInfo> = loans
            .into_iter()
            .enumerate()
            .map(|(idx, l)| {
                // `game.sii` stores `duration` in weeks (5 = 35d, 10 = 70d).
                let weeks = if l.duration <= 20 { l.duration.max(1) as u32 } else { (l.duration as u32 + 6) / 7 };
                let days_total = weeks * 7;
                let dp = exact_daily_payment(l.original_amount, l.interest_rate, weeks);
                let dprin = exact_daily_principal(l.original_amount, weeks).max(1);
                let days_remaining = if l.amount <= 0 {
                    0
                } else {
                    (((l.amount + dprin - 1) / dprin) as u32).min(days_total)
                };
                let days_paid = days_total.saturating_sub(days_remaining);
                let paid_amount = l.original_amount.saturating_sub(l.amount).max(0);
                LoanInfo {
                    index: idx,
                    amount: l.amount,
                    original_amount: l.original_amount,
                    paid_amount,
                    interest_rate: l.interest_rate,
                    duration: days_total as i32,
                    duration_weeks: weeks,
                    daily_payment: dp,
                    daily_principal: dprin,
                    days_total,
                    days_paid,
                    days_remaining,
                    total_repayment: dp * i64::from(days_total),
                    can_repay: false,
                }
            })
            .collect();
        if self.loans != next {
            self.loans = next;
            true
        } else {
            false
        }
    }

    /// Queues a new loan for offline save patching (used only when the game is closed).
    pub fn queue_loan(&mut self, amount: i64, days: i32, rate: f32) {
        if amount <= 0 || days <= 0 {
            return;
        }
        self.state.pending_loan = Some(PendingLoan { amount, days, rate });
        self.keep();
    }

    /// Cancels a queued offline loan.
    pub fn cancel_pending_loan(&mut self) {
        if self.state.pending_loan.take().is_some() {
            self.keep();
        }
    }

    /// Applies any pending offline loan to `game.sii` when the game is closed.
    pub fn try_apply_pending_loan(&mut self) -> bool {
        let Some(loan) = self.state.pending_loan.clone() else { return false };
        if westplay_setup::game::running() {
            return false;
        }
        let Some(path) = self.save_path.clone() else { return false };
        let Ok(bytes) = std::fs::read(&path) else { return false };
        let weeks = if loan.days <= 20 { loan.days.max(1) } else { (loan.days + 6) / 7 };
        match westplay_mapdata::save::add_loan(&bytes, loan.amount, loan.rate, weeks) {
            Ok(Some(updated)) => {
                let backup = path.with_extension("sii.westplay.bak");
                let _ = std::fs::copy(&path, &backup);
                if std::fs::write(&path, updated).is_ok() {
                    self.state.pending_loan = None;
                    if let Some(saved) = self.state.saved {
                        self.state.saved = Some(saved.saturating_add(loan.amount));
                    }
                    let dp = exact_daily_payment(loan.amount, loan.rate, weeks as u32);
                    let dprin = exact_daily_principal(loan.amount, weeks as u32).max(1);
                    let days_total = (weeks as u32) * 7;
                    self.loans.push(LoanInfo {
                        index: self.loans.len(),
                        amount: loan.amount,
                        original_amount: loan.amount,
                        paid_amount: 0,
                        interest_rate: loan.rate,
                        duration: days_total as i32,
                        duration_weeks: weeks as u32,
                        daily_payment: dp,
                        daily_principal: dprin,
                        days_total,
                        days_paid: 0,
                        days_remaining: days_total,
                        total_repayment: dp * i64::from(days_total),
                        can_repay: false,
                    });
                    self.keep();
                    crate::log!("[nupay] offline loan {} ({}w) written to {}", loan.amount, weeks, path.display());
                    return true;
                }
            }
            Ok(None) => {
                crate::log!("[nupay] pending loan skipped: {} is binary BSII", path.display());
            }
            Err(e) => {
                crate::log!("[nupay] failed to apply pending loan to {}: {e}", path.display());
            }
        }
        false
    }

    /// JSON message broadcast to phone & cabin pages.
    pub fn message(&self) -> String {
        let entries: Vec<&Entry> = self.state.entries.iter().rev().take(SENT_ENTRIES).collect();
        serde_json::json!({
            "type": "wallet",
            "balance": self.balance(),
            "currency": self.currency,
            "liveBank": self.live_bank,
            "bankUnlocked": self.bank_unlocked,
            "loanLimit": self.loan_limit,
            "availableCredit": self.available_credit,
            "minAfterRepay": self.min_after_repay,
            "entries": entries,
            "loans": self.loans,
            "offers": self.offers,
            "pendingLoan": self.state.pending_loan,
        })
        .to_string()
    }

    fn push(&mut self, entry: Entry) {
        self.fresh.push(entry.clone());
        self.state.entries.push(entry);
        let over = self.state.entries.len().saturating_sub(MOST_ENTRIES);
        self.state.entries.drain(..over);
    }

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

/// Host RPC controller for instant in-memory Bank actions (`Refresh`, `TakeLoan`, `RepayLoan`).
#[derive(Default)]
pub struct BankRpc {
    pending: Option<PendingBankRpc>,
}

struct PendingBankRpc {
    request: u32,
    id: u32,
    action: BankAction,
    index: usize,
    sent: Instant,
    /// Snapshot of amount/detail captured BEFORE executing the RPC so we can record a crisp Extrato entry on `Ok`.
    amount: i64,
    detail: String,
}

impl BankRpc {
    pub fn ask_refresh(&mut self, block: &BankBlock, id: u32, now: Instant) -> Option<String> {
        self.send(block, id, BankAction::Refresh, 0, 0, String::new(), now)
    }

    pub fn ask_take_loan(
        &mut self,
        block: &BankBlock,
        id: u32,
        offer_index: u32,
        amount: i64,
        detail: String,
        now: Instant,
    ) -> Option<String> {
        self.send(block, id, BankAction::TakeLoan, offer_index as usize, amount, detail, now)
    }

    pub fn ask_repay_loan(
        &mut self,
        block: &BankBlock,
        id: u32,
        loan_index: u32,
        amount: i64,
        detail: String,
        now: Instant,
    ) -> Option<String> {
        self.send(block, id, BankAction::RepayLoan, loan_index as usize, amount, detail, now)
    }

    #[allow(clippy::too_many_arguments)]
    fn send(
        &mut self,
        block: &BankBlock,
        id: u32,
        action: BankAction,
        index: usize,
        amount: i64,
        detail: String,
        now: Instant,
    ) -> Option<String> {
        let overtaken = self.pending.take().map(|p| bank_answer_msg(p.id, action_name(p.action), "noAnswer"));
        block.action.store(action as u32, Ordering::Relaxed);
        block.index.store(index as u32, Ordering::Relaxed);
        let request = block.request.load(Ordering::Relaxed).wrapping_add(1);
        block.request.store(request, Ordering::Release);
        self.pending = Some(PendingBankRpc { request, id, action, index, sent: now, amount, detail });
        overtaken
    }

    /// Polls the plugin's RPC response. On `Ok` for TakeLoan/RepayLoan, also appends an entry to `wallet`.
    pub fn poll(&mut self, block: &BankBlock, wallet: &mut Wallet, now: Instant) -> Option<String> {
        let pending = self.pending.as_ref()?;
        let done = block.answered.load(Ordering::Acquire) == pending.request;
        if !done {
            if now.duration_since(pending.sent) >= BANK_RPC_TIMEOUT {
                let p = self.pending.take()?;
                return Some(bank_answer_msg(p.id, action_name(p.action), "noAnswer"));
            }
            return None;
        }
        let p = self.pending.take()?;
        let answer = BankAnswer::from_code(block.answer.load(Ordering::Relaxed));
        let code = answer.map_or("failed", BankAnswer::name);
        // Immediately pull fresh live state published by the plugin.
        let _ = wallet.poll_live_bank(block);
        if answer == Some(BankAnswer::Ok) {
            match p.action {
                BankAction::TakeLoan if p.amount > 0 => {
                    wallet.record_bank_entry("loan", p.amount, &p.detail);
                }
                BankAction::RepayLoan if p.amount > 0 => {
                    wallet.record_bank_entry("loanRepay", -p.amount, &p.detail);
                }
                _ => {}
            }
            let _ = p.index;
        }
        Some(bank_answer_msg(p.id, action_name(p.action), code))
    }
}

fn action_name(a: BankAction) -> &'static str {
    match a {
        BankAction::Refresh => "refresh",
        BankAction::TakeLoan => "takeLoan",
        BankAction::RepayLoan => "repayLoan",
    }
}

fn bank_answer_msg(id: u32, action: &str, answer: &str) -> String {
    serde_json::json!({ "type": "bankAnswer", "id": id, "action": action, "answer": answer }).to_string()
}

fn exact_weekly_payment(original_amount: i64, interest_rate: f32, duration_weeks: u32) -> i64 {
    if original_amount <= 0 || duration_weeks == 0 {
        return 0;
    }
    let p = original_amount as f32;
    let n = duration_weeks as f32;
    if interest_rate > 0.001 {
        let r = interest_rate * (1.0f32 / 52.0f32);
        let factor = (1.0f32 + r).powf(n);
        let denom = factor - 1.0f32;
        if denom <= 0.0 {
            return (p / n).ceil() as i64;
        }
        (p * r * factor / denom).ceil() as i64
    } else {
        (p / n + 0.5f32).floor() as i64
    }
}

fn exact_daily_payment(original_amount: i64, interest_rate: f32, duration_weeks: u32) -> i64 {
    let w = exact_weekly_payment(original_amount, interest_rate, duration_weeks);
    ((w as f32) * (1.0f32 / 7.0f32)).ceil() as i64
}

fn exact_daily_principal(original_amount: i64, duration_weeks: u32) -> i64 {
    let w = exact_weekly_payment(original_amount, 0.0, duration_weeks);
    ((w as f32) * (1.0f32 / 7.0f32)).ceil() as i64
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

/// The currency the game `game` (its SDK id) counts its money in: ETS2's euros, else ATS's dollars.
fn currency_of(game: &str) -> &'static str {
    if game == "eut2" { "EUR" } else { "USD" }
}

#[cfg(test)]
mod tests {
    use westplay_shared::{FixedStr, LiveLoan, LiveLoanOffer, MoneyEvent};

    use super::*;

    fn wallet() -> Wallet {
        Wallet {
            path: None,
            state: State::default(),
            live_balance: None,
            live_bank: false,
            bank_unlocked: true,
            loan_limit: 0,
            available_credit: 0,
            min_after_repay: 0,
            loans: Vec::new(),
            offers: Vec::new(),
            save_path: None,
            seen: None,
            last_live_seq: 0,
            currency: "USD",
            fresh: Vec::new(),
            save_read: false,
        }
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
    fn live_bank_overrides_stale_save_balance_and_mirrors_offers() {
        let mut wallet = wallet();
        wallet.saved(10_000);
        let mut live = LiveBank {
            valid: 1,
            app_enabled: 1,
            balance: 250_500,
            loan_limit: 500_000,
            available_credit: 400_000,
            loans_count: 1,
            offers_count: 1,
            ..Default::default()
        };
        live.loans[0] = LiveLoan {
            amount: 80_000,
            original_amount: 100_000,
            daily_payment: 1_447,
            daily_principal: 1_429,
            duration_weeks: 10,
            interest_rate: 0.12,
            days_total: 70,
            days_paid: 14,
            days_remaining: 56,
            can_repay: 1,
            ..Default::default()
        };
        live.offers[0] = LiveLoanOffer {
            index: 0,
            amount: 100_000,
            daily_payment: 1_447,
            total_repayment: 101_290,
            duration_weeks: 10,
            interest_rate: 0.12,
            can_take: 1,
        };
        assert!(wallet.sync_live_bank(&live));
        assert_eq!(wallet.balance(), Some(250_500));
        // Stale save cannot overwrite live balance!
        assert!(!wallet.saved(10_000));
        assert_eq!(wallet.balance(), Some(250_500));
        assert_eq!(wallet.loans.len(), 1);
        assert_eq!(wallet.loans[0].days_paid, 14);
        assert_eq!(wallet.loans[0].days_remaining, 56);
        assert_eq!(wallet.offers.len(), 1);
    }
}
