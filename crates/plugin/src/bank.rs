//! Live in-memory Bank & Loans (Nupay) for ETS2 & ATS.
//!
//! Mirrors `economy_u->bank` (`money_account`, `loan_limit`, `app_enabled`, active `bank_loan_u`s, and live
//! `bank_data_u->loan_offers`) directly from process memory so Nupay's balance and loan options match the
//! game 1:1 in real time, and lets the player take or repay loans immediately on the main thread without
//! restarting the game.
//!
//! Zero hardcoded RVAs or struct offsets:
//! - Function pointers and globals are located dynamically from Prism3D's unique `.rdata` bank array symbols
//!   and `.pdata` unwinding.
//! - Struct field offsets come from Prism3D's `.data` `attribute_t` reflection tables.
//! - All memory reads use `safe_read` (`ReadProcessMemory`).
//! - Loan write actions (`TakeLoan`, `RepayLoan`) are disabled in TruckersMP multiplayer sessions.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use westplay_shared::{
    BankAction, BankAnswer, LIVE_LOANS_MAX, LIVE_OFFERS_MAX, LiveBank, LiveLoan, LiveLoanOffer,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::job_market::{PeImage, read_ptr_array, safe_read, valid_ptr};
use crate::{bridge, hooks, log, telemetry};

/// `create_bank_loan(factory_ptr) -> *mut u8` (allocates a new `bank_loan_u`).
type CreateLoanFn = unsafe extern "system" fn(*mut u8) -> *mut u8;
/// `bank_u::remaining_loan_limit(bank_u*) -> i64`.
type RemainingLimitFn = unsafe extern "system" fn(*mut u8) -> i64;
/// `bank_u::add_money(bank_u*, mail_ctrl*, amount: i64, notify: u8)`.
type AddMoneyFn = unsafe extern "system" fn(*mut u8, *mut u8, i64, u8);
/// `bank_u::pay_money(bank_u*, mail_ctrl*, amount: i64, notify: u8)`.
type PayMoneyFn = unsafe extern "system" fn(*mut u8, *mut u8, i64, u8);
/// `bank_u::can_repay_loan(bank_u*, loan_idx: usize) -> bool`.
type CanRepayFn = unsafe extern "system" fn(*mut u8, usize) -> bool;
/// `array_t<owner_ptr_t<bank_loan_u>>::grow(array_ptr, delta: usize) -> bool`.
type ArrayGrowFn = unsafe extern "system" fn(*mut u8, usize) -> bool;
/// `array_t<owner_ptr_t<bank_loan_u>>::remove(array_ptr, index: usize) -> *mut u8`.
type LoansRemoveFn = unsafe extern "system" fn(*mut u8, usize) -> *mut u8;

/// Poll live bank memory every N frames (~3 times/sec at 60 fps) when idle, or immediately on RPC.
const POLL_EVERY_FRAMES: u32 = 20;

#[derive(Clone, Copy, Debug)]
struct Offsets {
    // economy_u
    econ_bank: usize,
    econ_mail_ctrl: usize,
    econ_game_time: usize,
    // bank_u
    bank_money_account: usize,
    bank_loans: usize,
    bank_app_enabled: usize,
    bank_loan_limit: usize,
    bank_data: usize,
    bank_payment_timer: usize,
    // bank_data_u
    bdata_min_after_repay: usize,
    bdata_loan_offers: usize,
    // bank_loan_u
    loan_amount: usize,
    loan_original_amount: usize,
    loan_time_stamp: usize,
    loan_interest_rate: usize,
    loan_duration: usize,
    // bank_loan_data_u
    offer_amount: usize,
    offer_interest_rate: usize,
    offer_duration: usize,
}

struct Engine {
    econ_mgr_global: *const *mut u8,
    unit_type_idx_global: *const u32,
    unit_factories_global: *const *mut u8,
    create_loan_fn: CreateLoanFn,
    remaining_limit_fn: RemainingLimitFn,
    add_money_fn: AddMoneyFn,
    pay_money_fn: PayMoneyFn,
    can_repay_fn: CanRepayFn,
    array_grow_fn: ArrayGrowFn,
    loans_remove_fn: LoansRemoveFn,
    off: Offsets,
}

// SAFETY: pointers are into the game's executable image (fixed for process life) and used only on the main thread.
unsafe impl Send for Engine {}
// SAFETY: never accessed concurrently across threads.
unsafe impl Sync for Engine {}

static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();
static POLL_TICK: AtomicU32 = AtomicU32::new(0);
static LAST_LIVE: Mutex<Option<LiveBank>> = Mutex::new(None);

/// Serves periodic live bank synchronization and pending Nupay RPCs on `SCS_TELEMETRY_EVENT_FRAME_END`.
///
/// # Safety
/// Must be called only on the game's main thread during `SCS_TELEMETRY_EVENT_FRAME_END`.
pub unsafe fn serve() {
    let Some(bridge) = bridge::get() else { return };
    let block = &bridge.bank;
    let request = block.request.load(Ordering::Acquire);
    let has_rpc = request != block.answered.load(Ordering::Relaxed);

    let tick = POLL_TICK.fetch_add(1, Ordering::Relaxed);
    if !has_rpc && !tick.is_multiple_of(POLL_EVERY_FRAMES) {
        return;
    }

    hooks::contained("bank::serve", (), || {
        // SAFETY: we are inside the game process on the main thread.
        let engine = ENGINE.get_or_init(|| unsafe { locate() }).as_ref();
        if let Some(engine) = engine {
            if !has_rpc {
                // Periodic read-only mirror of live balance, active loans, and game loan offers.
                publish_live(engine, block, false);
                return;
            }
            let action = BankAction::from_code(block.action.load(Ordering::Acquire));
            let index = block.index.load(Ordering::Acquire) as usize;
            let answer = match action {
                Some(BankAction::Refresh) => {
                    if publish_live(engine, block, true) {
                        BankAnswer::Ok
                    } else {
                        BankAnswer::NotReady
                    }
                }
                Some(BankAction::TakeLoan) => {
                    if telemetry::truck().multiplayer {
                        BankAnswer::Multiplayer
                    } else {
                        // SAFETY: main thread, validated engine handles.
                        let ans = unsafe { take_loan(engine, index) };
                        publish_live(engine, block, true);
                        ans
                    }
                }
                Some(BankAction::RepayLoan) => {
                    if telemetry::truck().multiplayer {
                        BankAnswer::Multiplayer
                    } else {
                        // SAFETY: main thread, validated engine handles.
                        let ans = unsafe { repay_loan(engine, index) };
                        publish_live(engine, block, true);
                        ans
                    }
                }
                None => BankAnswer::Failed,
            };
            block.answer.store(answer as i32, Ordering::Relaxed);
            block.answered.store(request, Ordering::Release);
        } else if has_rpc {
            block.answer.store(BankAnswer::Unsupported as i32, Ordering::Relaxed);
            block.answered.store(request, Ordering::Release);
        }
    });
}

// ---------------------------------------------------------------------------
// Exact ETS2 / ATS loan payment math (verified from 0x140A34BD0 & 0x140827630)
// ---------------------------------------------------------------------------

/// Exact Prism3D weekly installment formula (`0x140A34BD0`):
/// - `r_week = interest_rate / 52.0`
/// - if `interest_rate > 0.001`: `ceil(amount * r_week * (1 + r_week)^weeks / ((1 + r_week)^weeks - 1))`
/// - else: `floor(amount / weeks + 0.5)`
pub(crate) fn weekly_payment(original_amount: i64, interest_rate: f32, duration_weeks: u32) -> i64 {
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

/// Exact Prism3D daily installment (`ceil(weekly / 7.0)`).
pub(crate) fn daily_payment(original_amount: i64, interest_rate: f32, duration_weeks: u32) -> i64 {
    let w = weekly_payment(original_amount, interest_rate, duration_weeks);
    ((w as f32) * (1.0f32 / 7.0f32)).ceil() as i64
}

/// Exact Prism3D daily principal deduction (`ceil(weekly(rate = 0) / 7.0)`).
pub(crate) fn daily_principal(original_amount: i64, duration_weeks: u32) -> i64 {
    let w = weekly_payment(original_amount, 0.0, duration_weeks);
    ((w as f32) * (1.0f32 / 7.0f32)).ceil() as i64
}

// ---------------------------------------------------------------------------
// Live snapshot reader & native take/repay loan execution
// ---------------------------------------------------------------------------

fn read_live_bank(engine: &Engine) -> Option<LiveBank> {
    let economy_u = safe_read::<usize>(engine.econ_mgr_global as usize)?;
    if !valid_ptr(economy_u) {
        return None;
    }
    let bank_u = safe_read::<usize>(economy_u + engine.off.econ_bank)?;
    if !valid_ptr(bank_u) {
        return None;
    }
    let game_time = safe_read::<u32>(economy_u + engine.off.econ_game_time)?;
    let balance = safe_read::<i64>(bank_u + engine.off.bank_money_account)?;
    let app_enabled = safe_read::<u8>(bank_u + engine.off.bank_app_enabled).unwrap_or(0) != 0;
    let loan_limit = safe_read::<i64>(bank_u + engine.off.bank_loan_limit).unwrap_or(0);
    let payment_timer = safe_read::<u32>(bank_u + engine.off.bank_payment_timer).unwrap_or(0);
    let bank_data_u = safe_read::<usize>(bank_u + engine.off.bank_data).unwrap_or(0);

    let min_after_repay = if valid_ptr(bank_data_u) {
        safe_read::<i32>(bank_data_u + engine.off.bdata_min_after_repay).unwrap_or(0)
    } else {
        0
    };

    let loan_ptrs = read_ptr_array(bank_u + engine.off.bank_loans, LIVE_LOANS_MAX).unwrap_or_default();
    let mut loans = [LiveLoan::default(); LIVE_LOANS_MAX];
    let mut loans_count = 0usize;
    let mut total_owed = 0i64;

    for &loan_ptr in &loan_ptrs {
        if !valid_ptr(loan_ptr) || loans_count >= LIVE_LOANS_MAX {
            continue;
        }
        let Some(amount) = safe_read::<i64>(loan_ptr + engine.off.loan_amount) else { continue };
        let original_amount = safe_read::<i64>(loan_ptr + engine.off.loan_original_amount).unwrap_or(amount).max(amount);
        let time_stamp = safe_read::<u32>(loan_ptr + engine.off.loan_time_stamp).unwrap_or(0);
        let interest_rate = safe_read::<f32>(loan_ptr + engine.off.loan_interest_rate).unwrap_or(0.0);
        let duration_weeks = safe_read::<u32>(loan_ptr + engine.off.loan_duration).unwrap_or(0);

        total_owed = total_owed.saturating_add(amount.max(0));
        let days_total = duration_weeks.saturating_mul(7);
        let dp = daily_payment(original_amount, interest_rate, duration_weeks);
        let dprin = daily_principal(original_amount, duration_weeks).max(1);
        let days_remaining = if amount <= 0 {
            0
        } else {
            (((amount + dprin - 1) / dprin) as u32).min(days_total)
        };
        let days_paid = days_total.saturating_sub(days_remaining);
        let can_repay = app_enabled && balance >= amount.saturating_add(i64::from(min_after_repay));

        loans[loans_count] = LiveLoan {
            amount,
            original_amount,
            daily_payment: dp,
            daily_principal: dprin,
            time_stamp,
            duration_weeks,
            interest_rate,
            days_total,
            days_paid,
            days_remaining,
            can_repay: u32::from(can_repay),
            _pad: 0,
        };
        loans_count += 1;
    }

    let available_credit = loan_limit.saturating_sub(total_owed).max(0);

    let mut offers = [LiveLoanOffer::default(); LIVE_OFFERS_MAX];
    let mut offers_count = 0usize;
    if valid_ptr(bank_data_u) {
        if let Some(offer_ptrs) = read_ptr_array(bank_data_u + engine.off.bdata_loan_offers, LIVE_OFFERS_MAX) {
            for (idx, &offer_ptr) in offer_ptrs.iter().enumerate() {
                if !valid_ptr(offer_ptr) || offers_count >= LIVE_OFFERS_MAX {
                    continue;
                }
                let amount = i64::from(safe_read::<u32>(offer_ptr + engine.off.offer_amount).unwrap_or(0));
                let interest_rate = safe_read::<f32>(offer_ptr + engine.off.offer_interest_rate).unwrap_or(0.0);
                let duration_weeks = safe_read::<u32>(offer_ptr + engine.off.offer_duration).unwrap_or(0);
                if amount <= 0 || duration_weeks == 0 {
                    continue;
                }
                let dp = daily_payment(amount, interest_rate, duration_weeks);
                let total_repayment = dp.saturating_mul(i64::from(duration_weeks.saturating_mul(7)));
                let can_take = app_enabled && amount <= available_credit;
                offers[offers_count] = LiveLoanOffer {
                    amount,
                    daily_payment: dp,
                    total_repayment,
                    index: idx as u32,
                    duration_weeks,
                    interest_rate,
                    can_take: u32::from(can_take),
                };
                offers_count += 1;
            }
        }
    }

    Some(LiveBank {
        valid: 1,
        app_enabled: u32::from(app_enabled),
        game_time,
        payment_timer,
        balance,
        loan_limit,
        available_credit,
        min_after_repay,
        loans_count: loans_count as u32,
        offers_count: offers_count as u32,
        _pad: 0,
        loans,
        offers,
    })
}

fn publish_live(engine: &Engine, block: &westplay_shared::BankBlock, force: bool) -> bool {
    let Some(live) = read_live_bank(engine) else { return false };
    let mut last = LAST_LIVE.lock().unwrap_or_else(|e| e.into_inner());
    if force || last.as_ref() != Some(&live) {
        block.live.write(&live);
        block.live_seq.fetch_add(1, Ordering::Release);
        *last = Some(live);
    }
    true
}

/// Takes loan `offer_index` from `bank_data_u->loan_offers` directly in memory, crediting `money_account`
/// and adding the new `bank_loan_u` to `bank_u->loans` on the game's main thread.
///
/// # Safety
/// Must be called on the game's main thread.
unsafe fn take_loan(engine: &Engine, offer_index: usize) -> BankAnswer {
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    let Some(bank_u) = safe_read::<usize>(economy_u + engine.off.econ_bank).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    let Some(mail_ctrl) = safe_read::<usize>(economy_u + engine.off.econ_mail_ctrl).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    if safe_read::<u8>(bank_u + engine.off.bank_app_enabled).unwrap_or(0) == 0 {
        return BankAnswer::Locked;
    }
    let Some(bank_data_u) = safe_read::<usize>(bank_u + engine.off.bank_data).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    let Some(offers) = read_ptr_array(bank_data_u + engine.off.bdata_loan_offers, LIVE_OFFERS_MAX) else {
        return BankAnswer::NotReady;
    };
    let Some(&offer_ptr) = offers.get(offer_index).filter(|&&p| valid_ptr(p)) else {
        return BankAnswer::InvalidIndex;
    };
    let offer_amount = i64::from(safe_read::<u32>(offer_ptr + engine.off.offer_amount).unwrap_or(0));
    let offer_rate = safe_read::<f32>(offer_ptr + engine.off.offer_interest_rate).unwrap_or(0.0);
    let offer_duration = safe_read::<u32>(offer_ptr + engine.off.offer_duration).unwrap_or(0);
    if offer_amount <= 0 || offer_duration == 0 {
        return BankAnswer::InvalidIndex;
    }

    // Check remaining credit limit via Prism3D's own `bank_u::remaining_loan_limit`.
    // SAFETY: main thread, validated `bank_u*`.
    let remaining = unsafe { (engine.remaining_limit_fn)(bank_u as *mut u8) };
    if remaining < offer_amount {
        log!("bank: take_loan({offer_index}) rejected — remaining limit {remaining} < {offer_amount}");
        return BankAnswer::LimitExceeded;
    }

    // Validate factory before allocating.
    let Some(type_idx) = safe_read::<u32>(engine.unit_type_idx_global as usize) else {
        return BankAnswer::Failed;
    };
    if type_idx == 4 {
        return BankAnswer::Failed;
    }
    let Some(factory) = safe_read::<usize>((engine.unit_factories_global as usize) + (type_idx as usize) * 8)
        .filter(|&p| valid_ptr(p))
    else {
        return BankAnswer::Failed;
    };

    // Grow `bank_u->loans` first so we never allocate/credit if growth fails.
    let loans_arr = bank_u + engine.off.bank_loans;
    // SAFETY: main thread, `loans_arr` is `bank_u->loans` (`array_t<owner_ptr_t<bank_loan_u>>`).
    let grew = unsafe { (engine.array_grow_fn)(loans_arr as *mut u8, 1) };
    if !grew {
        log!("bank: take_loan({offer_index}) failed — loans array grow returned false");
        return BankAnswer::Failed;
    }

    // Allocate `bank_loan_u` via Prism3D's native factory.
    // SAFETY: main thread, validated factory pointer from Prism3D's unit factory table.
    let loan_ptr = unsafe { (engine.create_loan_fn)(factory as *mut u8) } as usize;
    if !valid_ptr(loan_ptr) {
        log!("bank: take_loan({offer_index}) failed — create_loan returned null");
        return BankAnswer::Failed;
    }

    let game_time = safe_read::<u32>(economy_u + engine.off.econ_game_time).unwrap_or(0);
    // SAFETY: `loan_ptr` is a freshly allocated `bank_loan_u` on the game's main thread.
    // Mark owned + refcount 1 (`0x2000_0001`), matching `bank_ui` lines `0x11AF40E..0x11AF439`.
    unsafe {
        let flags = &*((loan_ptr + 8) as *const AtomicU32);
        flags.fetch_add(0x2000_0001, Ordering::AcqRel);
        *((loan_ptr + engine.off.loan_amount) as *mut i64) = offer_amount;
        *((loan_ptr + engine.off.loan_original_amount) as *mut i64) = offer_amount;
        *((loan_ptr + engine.off.loan_time_stamp) as *mut u32) = game_time;
        *((loan_ptr + engine.off.loan_interest_rate) as *mut f32) = offer_rate;
        *((loan_ptr + engine.off.loan_duration) as *mut u32) = offer_duration;

        // Place into the newly grown tail slot of `bank_u->loans`.
        let data_ptr = safe_read::<usize>(loans_arr + 8).unwrap_or(0);
        let len = safe_read::<u64>(loans_arr + 16).unwrap_or(0) as usize;
        if valid_ptr(data_ptr) && len > 0 {
            let slot = (data_ptr + (len - 1) * 8) as *mut usize;
            *slot = loan_ptr;
        }

        // Credit player's live `money_account` with in-game sound/notification (`notify = 1`).
        (engine.add_money_fn)(bank_u as *mut u8, mail_ctrl as *mut u8, offer_amount, 1);
    }

    log!(
        "bank: took live loan #{offer_index}: amount={offer_amount}, rate={offer_rate:.2}, weeks={offer_duration}"
    );
    BankAnswer::Ok
}

/// Repays active loan `loan_index` from `bank_u->loans` directly in memory on the game's main thread.
///
/// # Safety
/// Must be called on the game's main thread.
unsafe fn repay_loan(engine: &Engine, loan_index: usize) -> BankAnswer {
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    let Some(bank_u) = safe_read::<usize>(economy_u + engine.off.econ_bank).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    let Some(mail_ctrl) = safe_read::<usize>(economy_u + engine.off.econ_mail_ctrl).filter(|&p| valid_ptr(p)) else {
        return BankAnswer::NotReady;
    };
    if safe_read::<u8>(bank_u + engine.off.bank_app_enabled).unwrap_or(0) == 0 {
        return BankAnswer::Locked;
    }
    let loans_arr = bank_u + engine.off.bank_loans;
    let Some(loans) = read_ptr_array(loans_arr, LIVE_LOANS_MAX) else {
        return BankAnswer::NotReady;
    };
    let Some(&loan_ptr) = loans.get(loan_index).filter(|&&p| valid_ptr(p)) else {
        return BankAnswer::InvalidIndex;
    };

    // SAFETY: main thread, validated `bank_u*` and in-bounds `loan_index`.
    let can_repay = unsafe { (engine.can_repay_fn)(bank_u as *mut u8, loan_index) };
    if !can_repay {
        return BankAnswer::InsufficientFunds;
    }
    let Some(remaining_amount) = safe_read::<i64>(loan_ptr + engine.off.loan_amount) else {
        return BankAnswer::Failed;
    };

    // SAFETY: main thread, exact Prism3D Bank UI sequence (`pay_money` then `loans.remove(loan_index)`).
    unsafe {
        (engine.pay_money_fn)(bank_u as *mut u8, mail_ctrl as *mut u8, remaining_amount, 1);
        (engine.loans_remove_fn)(loans_arr as *mut u8, loan_index);
    }

    log!("bank: repaid live loan #{loan_index} ({remaining_amount})");
    BankAnswer::Ok
}

// ---------------------------------------------------------------------------
// Dynamic PE & Reflection Discovery (zero hardcoded RVAs or offsets)
// ---------------------------------------------------------------------------

fn find_bytes(hay: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    hay.get(start..)?.windows(needle.len()).position(|w| w == needle).map(|i| start + i)
}

unsafe fn locate() -> Option<Engine> {
    // SAFETY: main game executable handle mapped for process life.
    let base = unsafe { GetModuleHandleW(None) }.ok()?.0 as usize;
    // SAFETY: `base` is the mapped game executable.
    let img = unsafe { PeImage::of(base) }?;

    // 1. Locate Bank UI take/repay handler uniquely as the intersection of `bank_loan_u` and `bank_loan_data_u` xrefs.
    let s_loan_arr = img.find_rdata(
        b"??A?$array_t@V?$owner_ptr_t@Vbank_loan_u@prism@@@prism@@@prism@@QEAAAEAV?$owner_ptr_t@Vbank_loan_u@prism@@@1@_K@Z\0",
    )?;
    let s_offer_arr = img.find_rdata(
        b"??A?$array_t@V?$owner_ptr_t@Vbank_loan_data_u@prism@@@prism@@@prism@@QEBAAEBV?$owner_ptr_t@Vbank_loan_data_u@prism@@@1@_K@Z\0",
    )?;
    let roots_a: HashSet<usize> =
        img.find_lea_xrefs(s_loan_arr).into_iter().filter_map(|x| img.pdata_root(x)).collect();
    let roots_b: HashSet<usize> =
        img.find_lea_xrefs(s_offer_arr).into_iter().filter_map(|x| img.pdata_root(x)).collect();
    let mut common = roots_a.intersection(&roots_b).copied();
    let bank_ui = common.next()?;
    if common.next().is_some() {
        log!("bank: ambiguous bank_ui handler intersection");
        return None;
    }

    let text_off = bank_ui.checked_sub(img.text.0)?;
    let b = img.bytes(img.text).get(text_off..text_off + 0x280)?;

    // `mov rbx, [rip + g_economy_mgr]` -> `48 8B 1D <i32>`
    let i_econ = find_bytes(b, &[0x48, 0x8B, 0x1D], 0)?;
    let econ_disp = i32::from_le_bytes(b[i_econ + 3..i_econ + 7].try_into().unwrap()) as isize;
    let econ_mgr_global = (bank_ui + i_econ + 7).wrapping_add_signed(econ_disp);

    // `mov rcx, [r14 + bank_data_off]` -> `49 8B 4E <u8>`
    let i_bdata = find_bytes(b, &[0x49, 0x8B, 0x4E], i_econ)?;
    let bank_data_off = b[i_bdata + 3] as usize;

    // `mov eax, [rip + g_unit_type_idx]` -> `8B 05 <i32>`
    let i_tidx = find_bytes(b, &[0x8B, 0x05], i_bdata)?;
    let tidx_disp = i32::from_le_bytes(b[i_tidx + 2..i_tidx + 6].try_into().unwrap()) as isize;
    let unit_type_idx_global = (bank_ui + i_tidx + 6).wrapping_add_signed(tidx_disp);

    // `lea rcx, [rip + g_unit_factories]` -> `48 8D 0D <i32>`
    let i_fac = find_bytes(b, &[0x48, 0x8D, 0x0D], i_tidx)?;
    let fac_disp = i32::from_le_bytes(b[i_fac + 3..i_fac + 7].try_into().unwrap()) as isize;
    let unit_factories_global = (bank_ui + i_fac + 7).wrapping_add_signed(fac_disp);

    // `call create_loan` (`E8 <i32>`) right after factory load
    let i_create = find_bytes(b, &[0xE8], i_fac + 7)?;
    let create_disp = i32::from_le_bytes(b[i_create + 1..i_create + 5].try_into().unwrap()) as isize;
    let create_loan_va = (bank_ui + i_create + 5).wrapping_add_signed(create_disp);

    // `cmp byte ptr [r14 + app_enabled], 0` -> `41 80 7E` followed by 3 sequential calls:
    // `remaining_limit`, `add_money`, `array_grow`.
    let i_en = find_bytes(b, &[0x41, 0x80, 0x7E], i_create + 5)?;
    let mut take_calls = [0usize; 3];
    let mut pos = i_en;
    for slot in &mut take_calls {
        let p = find_bytes(b, &[0xE8], pos)?;
        let disp = i32::from_le_bytes(b[p + 1..p + 5].try_into().unwrap()) as isize;
        *slot = (bank_ui + p + 5).wrapping_add_signed(disp);
        pos = p + 5;
    }

    // Repay branch: `cmp edx, 0x12C` (`81 FA 2C 01 00 00`), then `41 80 7E` followed by 3 sequential calls:
    // `can_repay`, `pay_money`, `loans_remove`.
    let i_repay = find_bytes(b, &[0x81, 0xFA, 0x2C, 0x01, 0x00, 0x00], pos)?;
    let i_ren = find_bytes(b, &[0x41, 0x80, 0x7E], i_repay)?;
    let mut repay_calls = [0usize; 3];
    let mut rpos = i_ren;
    for slot in &mut repay_calls {
        let p = find_bytes(b, &[0xE8], rpos)?;
        let disp = i32::from_le_bytes(b[p + 1..p + 5].try_into().unwrap()) as isize;
        *slot = (bank_ui + p + 5).wrapping_add_signed(disp);
        rpos = p + 5;
    }

    // 2. Dynamic `.data` reflection tables (zero hardcoded struct offsets).
    let econ_tbl = img.find_attr_table(&["bank", "mail_ctrl", "game_time", "player", "companies"])?;
    let bank_tbl = img.find_exact_attr_table(
        &["money_account", "loans", "app_enabled", "loan_limit", "payment_timer"],
        &[],
    )?;
    let bdata_tbl = img.find_exact_attr_table(&["min_after_loan_repay", "loan_offers"], &[])?;
    let loan_tbl = img.find_exact_attr_table(
        &["amount", "original_amount", "time_stamp", "interest_rate", "duration"],
        &[],
    )?;
    let offer_tbl = img.find_exact_attr_table(&["amount", "interest_rate", "duration"], &["original_amount"])?;

    let off = Offsets {
        econ_bank: *econ_tbl.get("bank")?,
        econ_mail_ctrl: *econ_tbl.get("mail_ctrl")?,
        econ_game_time: *econ_tbl.get("game_time")?,
        bank_money_account: *bank_tbl.get("money_account")?,
        bank_loans: *bank_tbl.get("loans")?,
        bank_app_enabled: *bank_tbl.get("app_enabled")?,
        bank_loan_limit: *bank_tbl.get("loan_limit")?,
        bank_data: bank_data_off,
        bank_payment_timer: *bank_tbl.get("payment_timer")?,
        bdata_min_after_repay: *bdata_tbl.get("min_after_loan_repay")?,
        bdata_loan_offers: *bdata_tbl.get("loan_offers")?,
        loan_amount: *loan_tbl.get("amount")?,
        loan_original_amount: *loan_tbl.get("original_amount")?,
        loan_time_stamp: *loan_tbl.get("time_stamp")?,
        loan_interest_rate: *loan_tbl.get("interest_rate")?,
        loan_duration: *loan_tbl.get("duration")?,
        offer_amount: *offer_tbl.get("amount")?,
        offer_interest_rate: *offer_tbl.get("interest_rate")?,
        offer_duration: *offer_tbl.get("duration")?,
    };

    log!(
        "bank: ready! econ=+{:#x}, create=+{:#x}, add_money=+{:#x}, pay_money=+{:#x}, can_repay=+{:#x}, remove=+{:#x}, bank_data=+{:#x}",
        econ_mgr_global - base,
        create_loan_va - base,
        take_calls[1] - base,
        repay_calls[1] - base,
        repay_calls[0] - base,
        repay_calls[2] - base,
        off.bank_data,
    );

    // SAFETY: all function entry points verified by unique Prism3D symbols + `.text` call graph.
    Some(Engine {
        econ_mgr_global: econ_mgr_global as *const *mut u8,
        unit_type_idx_global: unit_type_idx_global as *const u32,
        unit_factories_global: unit_factories_global as *const *mut u8,
        create_loan_fn: unsafe { std::mem::transmute::<usize, CreateLoanFn>(create_loan_va) },
        remaining_limit_fn: unsafe { std::mem::transmute::<usize, RemainingLimitFn>(take_calls[0]) },
        add_money_fn: unsafe { std::mem::transmute::<usize, AddMoneyFn>(take_calls[1]) },
        array_grow_fn: unsafe { std::mem::transmute::<usize, ArrayGrowFn>(take_calls[2]) },
        can_repay_fn: unsafe { std::mem::transmute::<usize, CanRepayFn>(repay_calls[0]) },
        pay_money_fn: unsafe { std::mem::transmute::<usize, PayMoneyFn>(repay_calls[1]) },
        loans_remove_fn: unsafe { std::mem::transmute::<usize, LoansRemoveFn>(repay_calls[2]) },
        off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ets2_ats_loan_daily_installments() {
        // Classic ETS2 loans:
        // €10,000 @ 18%, 5 weeks (35 days) -> €289/day
        assert_eq!(daily_payment(10_000, 0.18, 5), 289);
        // €100,000 @ 12%, 10 weeks (70 days) -> €1,447/day
        assert_eq!(daily_payment(100_000, 0.12, 10), 1_447);
        // €400,000 @ 12%, 10 weeks (70 days) -> €5,788/day
        assert_eq!(daily_payment(400_000, 0.12, 10), 5_788);
        // Zero interest -> simple division
        assert_eq!(daily_principal(100_000, 10), 1_429);
    }
}
