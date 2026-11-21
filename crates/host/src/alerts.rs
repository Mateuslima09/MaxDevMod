//! The phone's notifications of the game and of MaxDevMod (owner 2026-11-20: "payments, those things, not only
//! TikTok"): money in and out (Nupay's account, wallet.rs), fines, the truck's fuel warning and its damage, the
//! driver's rest, a new job and its deadline. Each as the TikTok ones are (tiktok.rs): a `phoneNotification` for the
//! phone's page, its top showing for it while it's put away. Each group can be turned off in the phone's Settings
//! (the page says which, `HandsetMessage::Alerts`), and what repeats is told once.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use serde::Serialize;
use westplay_shared::{JobInfo, Telemetry};

use crate::wallet::Entry;

/// The groups the phone's Settings turn on and off (the page's `AlertGroup`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// Money in: a job paid, more at the save.
    MoneyIn,
    /// Money out: tolls, ferries, trains, fuel, a job's penalty, less at the save (repairs, purchases).
    MoneyOut,
    Fine,
    Fuel,
    Damage,
    Rest,
    Job,
}

impl Group {
    pub const ALL: [Group; 7] =
        [Self::MoneyIn, Self::MoneyOut, Self::Fine, Self::Fuel, Self::Damage, Self::Rest, Self::Job];

    pub fn name(self) -> &'static str {
        match self {
            Self::MoneyIn => "moneyIn",
            Self::MoneyOut => "moneyOut",
            Self::Fine => "fine",
            Self::Fuel => "fuel",
            Self::Damage => "damage",
            Self::Rest => "rest",
            Self::Job => "job",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|group| group.name() == name)
    }
}

/// One notification: the app it's of (its icon and name on the banner), what it is (`kind`, the page words it),
/// and what it tells: a sum (in the game's `currency`), a number (a percent, minutes), about what.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    #[serde(skip)]
    pub group: Group,
    pub app: &'static str,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// How many of the same, told as one (tolls one after another).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// A sum in the pump's own money (cents and all), its sign in `detail`: the fuel owed to the attendant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

impl Alert {
    pub fn new(group: Group, app: &'static str, kind: &'static str) -> Self {
        Self { group, app, kind, amount: None, currency: None, value: None, detail: None, count: None, total: None }
    }

    /// `{"type":"phoneNotification","app":"nupay","kind":"delivered","amount":21469,…}`.
    pub fn message(&self) -> String {
        let mut message = serde_json::to_value(self).unwrap_or_default();
        message["type"] = "phoneNotification".into();
        message.to_string()
    }
}

/// The wallet's new entries (`Wallet::take_fresh`) as notifications, those of a kind together told as one (a sum
/// and how many: tolls, a save's adjustment beside a job).
pub fn from_entries(entries: &[Entry], currency: &'static str) -> Vec<Alert> {
    let mut alerts: Vec<Alert> = Vec::new();
    for entry in entries {
        if entry.amount == 0 {
            continue;
        }
        let (group, kind) = match entry.kind.as_str() {
            "delivered" => (Group::MoneyIn, "delivered"),
            "cancelled" => (Group::MoneyOut, "cancelled"),
            "fined" => (Group::Fine, "fined"),
            "toll" => (Group::MoneyOut, "toll"),
            "ferry" => (Group::MoneyOut, "ferry"),
            "train" => (Group::MoneyOut, "train"),
            "fuel" => (Group::MoneyOut, "fuel"),
            _ if entry.amount > 0 => (Group::MoneyIn, "otherIn"),
            _ => (Group::MoneyOut, "otherOut"),
        };
        if let Some(same) = alerts.iter_mut().find(|alert| alert.kind == kind) {
            *same.amount.get_or_insert(0) += entry.amount;
            *same.count.get_or_insert(1) += 1;
            continue;
        }
        let mut alert = Alert::new(group, "nupay", kind);
        alert.amount = Some(entry.amount);
        alert.currency = Some(currency);
        alert.detail = (!entry.detail.is_empty()).then(|| entry.detail.clone());
        alerts.push(alert);
    }
    alerts
}

/// The fuel filled by hand owed to the pump's attendant (`PluginBlock::fuel_owed`, owner 2026-11-20: "a notice to
/// pay"): its total in the pump's money, its sign.
pub fn pay_attendant(total: f32, sign: &str) -> Alert {
    let mut alert = Alert::new(Group::Fuel, "nupay", "payAttendant");
    alert.total = Some((f64::from(total) * 100.0).round() / 100.0);
    alert.detail = Some(sign.to_owned());
    alert
}

/// Each new bill owed told once: the last one's number.
#[derive(Debug, Default)]
pub struct Owed {
    told: u32,
}

impl Owed {
    /// The bill owed now (its id 0: none): a notification for it if it's new.
    pub fn check(&mut self, bill: &westplay_shared::FuelBill) -> Option<Alert> {
        if bill.id == 0 || bill.id == self.told {
            return None;
        }
        self.told = bill.id;
        Some(pay_attendant(bill.total, bill.sign.as_str()))
    }
}

/// One of each kind, made up, to see and hear them (the Settings' "test").
pub fn samples(currency: &'static str) -> Vec<Alert> {
    let money = |group, kind, amount: i64, detail: &str| {
        let mut alert = Alert::new(group, "nupay", kind);
        (alert.amount, alert.currency) = (Some(amount), Some(currency));
        alert.detail = (!detail.is_empty()).then(|| detail.to_owned());
        alert
    };
    let with_value = |group, app, kind, value: i64, detail: Option<&str>| {
        let mut alert = Alert::new(group, app, kind);
        alert.value = Some(value);
        alert.detail = detail.map(str::to_owned);
        alert
    };
    vec![
        money(Group::MoneyIn, "delivered", 21_469, "Tomates → Sacramento"),
        money(Group::MoneyOut, "toll", -12, ""),
        money(Group::Fine, "fined", -300, "speeding"),
        Alert::new(Group::Fuel, "truck", "fuelLow"),
        pay_attendant(130.5, "R$"),
        with_value(Group::Damage, "truck", "damage", 25, Some("engine")),
        with_value(Group::Rest, "truck", "rest", 60, None),
        {
            let mut alert = Alert::new(Group::Job, "freight", "jobNew");
            alert.detail = Some("Tomates → Sacramento".into());
            alert
        },
    ]
}

/// The groups turned off in the phone's Settings: their notifications aren't told (nor the phone's top shown).
#[derive(Debug, Default)]
pub struct Filter {
    off: HashSet<Group>,
}

impl Filter {
    /// The groups named off by the page (names not known left out).
    pub fn set_off(&mut self, names: &[String]) {
        self.off = names.iter().filter_map(|name| Group::from_name(name)).collect();
    }

    pub fn allows(&self, alert: &Alert) -> bool {
        !self.off.contains(&alert.group)
    }
}

/// The wear past which the damage is told, each once on the way up.
const WEAR_STEPS: [f32; 4] = [0.10, 0.25, 0.50, 0.75];
/// A crash: the wear up this much at once, the truck moving just before.
const CRASH_WEAR: f32 = 0.02;
/// A repair: the wear down this much, the same truck.
const REPAIR_WEAR: f32 = 0.05;
/// Moving: faster than this (m/s) lately, as damage only comes driving (a truck changed or loaded doesn't).
const MOVING: f32 = 1.5;
const MOVING_LATELY: Duration = Duration::from_secs(4);
/// The least between two crashes told.
const CRASH_AGAIN: Duration = Duration::from_secs(20);
/// The least between two fuel warnings told (the warning flickering at its edge).
const FUEL_AGAIN: Duration = Duration::from_secs(10 * 60);
/// The rest told this many game minutes before it's due, and again when it is; told again after a rest (the time
/// to it back above `REST_RESET`).
const REST_SOON: i32 = 60;
const REST_RESET: i32 = 120;
/// The job's deadline told this many game minutes before it ends, and as it passes.
const DEADLINE_SOON: i64 = 60;
/// After the game starts telling (or the truck changes), how long only to look: what's there already isn't news.
const SETTLE: Duration = Duration::from_secs(8);

/// Watches the telemetry and the job for what's worth a notification, each told once.
#[derive(Debug, Default)]
pub struct Watch {
    started: Option<Instant>,
    truck: String,
    moving_at: Option<Instant>,
    wear: Option<f32>,
    /// How many of [`WEAR_STEPS`] the wear is past, as told.
    wear_step: usize,
    crash_at: Option<Instant>,
    fuel_warned: bool,
    fuel_at: Option<Instant>,
    rest_soon_told: bool,
    rest_now_told: bool,
    /// The job seen last (its cargo, from, to), and whether its deadline was told near, and passed.
    job: Option<(String, String, String)>,
    deadline_soon_told: bool,
    deadline_passed_told: bool,
}

impl Watch {
    /// The game stopped telling (closed): everything seen afresh when it's back.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// What's new in `telemetry` and `job` (the player's truck `truck`, its id), at `now`.
    pub fn tick(&mut self, telemetry: &Telemetry, job: Option<&JobInfo>, truck: &str, now: Instant) -> Vec<Alert> {
        let mut alerts = Vec::new();
        if truck != self.truck {
            // Another truck: its wear its own, seen afresh.
            self.truck = truck.to_owned();
            self.wear = None;
            self.started = Some(now);
        }
        let started = *self.started.get_or_insert(now);
        let settled = now.duration_since(started) >= SETTLE;
        if telemetry.speed.abs() > MOVING {
            self.moving_at = Some(now);
        }
        let moving = self.moving_at.is_some_and(|at| now.duration_since(at) <= MOVING_LATELY);

        // The truck's damage: the worst of its parts.
        let (part, wear) = telemetry
            .wear
            .iter()
            .copied()
            .enumerate()
            .fold((0, 0.0f32), |worst, (part, wear)| if wear > worst.1 { (part, wear) } else { worst });
        let step = WEAR_STEPS.iter().filter(|&&step| wear >= step).count();
        match self.wear {
            Some(before) if settled && !telemetry.paused => {
                if step > self.wear_step && moving {
                    let mut alert = Alert::new(Group::Damage, "truck", "damage");
                    alert.value = Some((wear * 100.0).round() as i64);
                    alert.detail = Some(PARTS[part].into());
                    alerts.push(alert);
                    self.crash_at = Some(now);
                } else if wear - before >= CRASH_WEAR
                    && moving
                    && self.crash_at.is_none_or(|at| now.duration_since(at) >= CRASH_AGAIN)
                {
                    let mut alert = Alert::new(Group::Damage, "truck", "crash");
                    alert.value = Some((wear * 100.0).round() as i64);
                    alert.detail = Some(PARTS[part].into());
                    alerts.push(alert);
                    self.crash_at = Some(now);
                } else if before - wear >= REPAIR_WEAR && !moving {
                    let mut alert = Alert::new(Group::Damage, "truck", "repaired");
                    alert.value = Some((wear * 100.0).round() as i64);
                    alerts.push(alert);
                }
                self.wear_step = step;
            }
            _ => self.wear_step = step,
        }
        self.wear = Some(wear);

        // The truck's own fuel warning, as it comes on.
        if telemetry.fuel_warning && telemetry.electric_enabled {
            if !self.fuel_warned && settled && self.fuel_at.is_none_or(|at| now.duration_since(at) >= FUEL_AGAIN) {
                let mut alert = Alert::new(Group::Fuel, "truck", "fuelLow");
                alert.value = (telemetry.fuel_range > 0.0).then(|| telemetry.fuel_range.round() as i64);
                alerts.push(alert);
                self.fuel_at = Some(now);
            }
            self.fuel_warned = true;
        } else if !telemetry.fuel_warning {
            self.fuel_warned = false;
        }

        // The driver's rest.
        if telemetry.rest_known {
            let minutes = telemetry.rest_stop;
            if minutes > REST_RESET {
                (self.rest_soon_told, self.rest_now_told) = (false, false);
            } else if !settled || telemetry.paused {
                // Already due as the game came: not told; due again after a rest.
                if !settled {
                    (self.rest_soon_told, self.rest_now_told) = (minutes <= REST_SOON, minutes <= 0);
                }
            } else if minutes <= 0 && !self.rest_now_told {
                alerts.push(Alert::new(Group::Rest, "truck", "restNow"));
                (self.rest_soon_told, self.rest_now_told) = (true, true);
            } else if minutes <= REST_SOON && !self.rest_soon_told {
                let mut alert = Alert::new(Group::Rest, "truck", "rest");
                alert.value = Some(i64::from(minutes));
                alerts.push(alert);
                self.rest_soon_told = true;
            }
        }

        // A new job, and its deadline.
        match job.filter(|job| job.active) {
            Some(job) => {
                let id = (
                    job.cargo.as_str().to_owned(),
                    job.source_city.as_str().to_owned(),
                    job.destination_city.as_str().to_owned(),
                );
                if self.job.as_ref() != Some(&id) {
                    if self.job.is_some() || settled {
                        let mut alert = Alert::new(Group::Job, "freight", "jobNew");
                        alert.detail = Some(format!("{} → {}", id.0, id.2));
                        alert.amount = (job.income > 0).then(|| job.income as i64);
                        alerts.push(alert);
                    }
                    self.job = Some(id);
                    (self.deadline_soon_told, self.deadline_passed_told) = (false, false);
                    if !settled {
                        // Seen as the game came: its deadline as it already is isn't news either.
                        let left = deadline_left(job, telemetry);
                        self.deadline_soon_told = left.is_some_and(|left| left <= DEADLINE_SOON);
                        self.deadline_passed_told = left.is_some_and(|left| left <= 0);
                    }
                }
                if settled
                    && !telemetry.paused
                    && let Some(left) = deadline_left(job, telemetry)
                {
                    if left <= 0 && !self.deadline_passed_told {
                        alerts.push(Alert::new(Group::Job, "freight", "jobLate"));
                        (self.deadline_soon_told, self.deadline_passed_told) = (true, true);
                    } else if left <= DEADLINE_SOON && !self.deadline_soon_told {
                        let mut alert = Alert::new(Group::Job, "freight", "jobDeadline");
                        alert.value = Some(left);
                        alerts.push(alert);
                        self.deadline_soon_told = true;
                    }
                }
            }
            // Delivered or let go: the next is new, even the same cargo again.
            None if settled => self.job = None,
            None => {}
        }
        alerts
    }
}

/// The parts of `Telemetry::wear`, as the page names them.
const PARTS: [&str; 5] = ["engine", "transmission", "cabin", "chassis", "wheels"];

/// Game minutes left until `job`'s delivery window ends; `None` with no deadline (a special transport, a time not
/// known).
fn deadline_left(job: &JobInfo, telemetry: &Telemetry) -> Option<i64> {
    let (due, now) = (i64::from(job.delivery_time), i64::from(telemetry.game_time));
    let left = due - now;
    // No time yet, none at all (the SDK's "never": a u32's most), or weeks away: not a deadline to tell.
    (job.delivery_time != 0 && job.delivery_time != u32::MAX && now != 0 && left < 60 * 24 * 14).then_some(left)
}

#[cfg(test)]
mod tests {
    use westplay_shared::FixedStr;

    use super::*;

    fn entry(kind: &str, amount: i64, detail: &str) -> Entry {
        Entry { at: 0, game_time: None, kind: kind.into(), amount, detail: detail.into() }
    }

    #[test]
    fn money_is_told_by_kind_the_same_together() {
        let alerts = from_entries(
            &[
                entry("delivered", 21_469, "Tomatoes → Reno"),
                entry("toll", -10, ""),
                entry("toll", -12, ""),
                entry("fined", -300, "speeding"),
                entry("other", -1_200, ""),
                entry("other", 0, ""),
            ],
            "USD",
        );
        let told: Vec<_> = alerts.iter().map(|alert| (alert.kind, alert.amount, alert.count)).collect();
        assert_eq!(
            told,
            [
                ("delivered", Some(21_469), None),
                ("toll", Some(-22), Some(2)),
                ("fined", Some(-300), None),
                ("otherOut", Some(-1_200), None),
            ]
        );
        assert_eq!(alerts[0].group, Group::MoneyIn);
        assert_eq!(alerts[2].group, Group::Fine);
        assert_eq!(alerts[3].group, Group::MoneyOut);
        let message: serde_json::Value = serde_json::from_str(&alerts[0].message()).unwrap();
        assert_eq!(message["type"], "phoneNotification");
        assert_eq!(message["app"], "nupay");
        assert_eq!(message["currency"], "USD");
        assert_eq!(message["detail"], "Tomatoes → Reno");
        assert!(message.get("group").is_none() && message.get("value").is_none());
    }

    #[test]
    fn a_bill_owed_is_told_once() {
        let mut owed = Owed::default();
        let bill = |id, total| westplay_shared::FuelBill {
            id,
            total,
            sign: westplay_shared::FixedStr::new("R$"),
            ..westplay_shared::FuelBill::default()
        };
        assert!(owed.check(&bill(0, 0.0)).is_none());
        let alert = owed.check(&bill(1, 130.456)).unwrap();
        let message: serde_json::Value = serde_json::from_str(&alert.message()).unwrap();
        assert_eq!((message["kind"].as_str(), message["total"].as_f64()), (Some("payAttendant"), Some(130.46)));
        assert_eq!(message["detail"], "R$");
        assert!(owed.check(&bill(1, 130.456)).is_none());
        assert!(owed.check(&bill(0, 0.0)).is_none());
        assert!(owed.check(&bill(2, 50.0)).is_some());
    }

    #[test]
    fn groups_turned_off_arent_told() {
        let mut filter = Filter::default();
        let fine = &from_entries(&[entry("fined", -300, "")], "USD")[0];
        assert!(filter.allows(fine));
        filter.set_off(&["fine".into(), "nonsense".into()]);
        assert!(!filter.allows(fine));
        assert!(filter.allows(&from_entries(&[entry("toll", -5, "")], "USD")[0]));
        filter.set_off(&[]);
        assert!(filter.allows(fine));
        for group in Group::ALL {
            assert_eq!(Group::from_name(group.name()), Some(group));
        }
        // The test notifications: one of each group.
        let samples = samples("EUR");
        assert!(Group::ALL.iter().all(|group| samples.iter().any(|alert| alert.group == *group)));
    }

    /// A watch past its first look, the truck driving.
    fn watching(at: Instant) -> (Watch, Telemetry) {
        let mut watch = Watch::default();
        let telemetry = Telemetry { speed: 20.0, electric_enabled: true, ..Telemetry::default() };
        assert!(watch.tick(&telemetry, None, "truck", at).is_empty());
        assert!(watch.tick(&telemetry, None, "truck", at + SETTLE).is_empty());
        (watch, telemetry)
    }

    #[test]
    fn damage_is_told_at_each_step_and_a_crash_once() {
        let at = Instant::now();
        let (mut watch, mut telemetry) = watching(at);
        let mut t = at + SETTLE;
        let mut tick = |watch: &mut Watch, telemetry: &Telemetry| {
            t += Duration::from_secs(1);
            watch.tick(telemetry, None, "truck", t).into_iter().map(|alert| (alert.kind, alert.value)).collect::<Vec<_>>()
        };
        telemetry.wear[2] = 0.03;
        assert_eq!(tick(&mut watch, &telemetry), [("crash", Some(3))]);
        // Again at once: not told again.
        telemetry.wear[2] = 0.06;
        assert!(tick(&mut watch, &telemetry).is_empty());
        telemetry.wear[2] = 0.11;
        assert_eq!(tick(&mut watch, &telemetry), [("damage", Some(11))]);
        // Slowly worse, past no step: nothing.
        telemetry.wear[2] = 0.115;
        assert!(tick(&mut watch, &telemetry).is_empty());
        // Repaired, standing.
        telemetry.speed = 0.0;
        for _ in 0..5 {
            tick(&mut watch, &telemetry);
        }
        telemetry.wear = [0.0; 5];
        assert_eq!(tick(&mut watch, &telemetry), [("repaired", Some(0))]);
    }

    #[test]
    fn a_damaged_truck_as_the_game_comes_or_changes_isnt_news() {
        let at = Instant::now();
        let mut watch = Watch::default();
        let mut telemetry = Telemetry { speed: 20.0, wear: [0.6, 0.0, 0.0, 0.0, 0.0], ..Telemetry::default() };
        assert!(watch.tick(&telemetry, None, "a", at).is_empty());
        assert!(watch.tick(&telemetry, None, "a", at + SETTLE).is_empty());
        // Another truck, standing in the garage: its wear isn't a crash.
        telemetry.speed = 0.0;
        telemetry.wear = [0.0, 0.0, 0.0, 0.9, 0.0];
        assert!(watch.tick(&telemetry, None, "b", at + SETTLE * 3).is_empty());
        assert!(watch.tick(&telemetry, None, "b", at + SETTLE * 5).is_empty());
    }

    #[test]
    fn the_fuel_warning_is_told_as_it_comes_on() {
        let at = Instant::now();
        let (mut watch, mut telemetry) = watching(at);
        let t = at + SETTLE;
        telemetry.fuel_warning = true;
        telemetry.fuel_range = 84.4;
        let told = watch.tick(&telemetry, None, "truck", t + Duration::from_secs(1));
        assert_eq!(told.iter().map(|alert| (alert.kind, alert.value)).collect::<Vec<_>>(), [("fuelLow", Some(84))]);
        assert!(watch.tick(&telemetry, None, "truck", t + Duration::from_secs(2)).is_empty());
        // Flickering off and on: not again so soon.
        telemetry.fuel_warning = false;
        watch.tick(&telemetry, None, "truck", t + Duration::from_secs(3));
        telemetry.fuel_warning = true;
        assert!(watch.tick(&telemetry, None, "truck", t + Duration::from_secs(4)).is_empty());
        // Filled up, and low again much later: told.
        telemetry.fuel_warning = false;
        watch.tick(&telemetry, None, "truck", t + FUEL_AGAIN);
        telemetry.fuel_warning = true;
        assert_eq!(watch.tick(&telemetry, None, "truck", t + FUEL_AGAIN * 2).len(), 1);
    }

    #[test]
    fn the_rest_is_told_soon_and_due_then_again_after_a_rest() {
        let at = Instant::now();
        let (mut watch, mut telemetry) = watching(at);
        let mut t = at + SETTLE;
        let mut kinds = |watch: &mut Watch, telemetry: &Telemetry| {
            t += Duration::from_secs(1);
            watch.tick(telemetry, None, "truck", t).into_iter().map(|alert| alert.kind).collect::<Vec<_>>()
        };
        telemetry.rest_known = true;
        telemetry.rest_stop = 300;
        assert!(kinds(&mut watch, &telemetry).is_empty());
        telemetry.rest_stop = 59;
        assert_eq!(kinds(&mut watch, &telemetry), ["rest"]);
        telemetry.rest_stop = 30;
        assert!(kinds(&mut watch, &telemetry).is_empty());
        telemetry.rest_stop = -5;
        assert_eq!(kinds(&mut watch, &telemetry), ["restNow"]);
        assert!(kinds(&mut watch, &telemetry).is_empty());
        telemetry.rest_stop = 660;
        assert!(kinds(&mut watch, &telemetry).is_empty());
        telemetry.rest_stop = 60;
        assert_eq!(kinds(&mut watch, &telemetry), ["rest"]);
    }

    fn job(cargo: &str, due: u32) -> JobInfo {
        JobInfo {
            active: true,
            cargo: FixedStr::new(cargo),
            source_city: FixedStr::new("Reno"),
            destination_city: FixedStr::new("Sacramento"),
            delivery_time: due,
            income: 9_000,
            ..JobInfo::default()
        }
    }

    #[test]
    fn a_new_job_and_its_deadline_are_told_once() {
        let at = Instant::now();
        let (mut watch, mut telemetry) = watching(at);
        let mut t = at + SETTLE;
        let mut kinds = |watch: &mut Watch, telemetry: &Telemetry, job: Option<&JobInfo>| {
            t += Duration::from_secs(1);
            watch.tick(telemetry, job, "truck", t).into_iter().map(|alert| alert.kind).collect::<Vec<_>>()
        };
        telemetry.game_time = 1_000;
        let tomatoes = job("Tomatoes", 1_500);
        assert_eq!(kinds(&mut watch, &telemetry, Some(&tomatoes)), ["jobNew"]);
        assert!(kinds(&mut watch, &telemetry, Some(&tomatoes)).is_empty());
        telemetry.game_time = 1_450;
        assert_eq!(kinds(&mut watch, &telemetry, Some(&tomatoes)), ["jobDeadline"]);
        telemetry.game_time = 1_480;
        assert!(kinds(&mut watch, &telemetry, Some(&tomatoes)).is_empty());
        telemetry.game_time = 1_501;
        assert_eq!(kinds(&mut watch, &telemetry, Some(&tomatoes)), ["jobLate"]);
        assert!(kinds(&mut watch, &telemetry, Some(&tomatoes)).is_empty());
        // Delivered; the same cargo again is a new job.
        assert!(kinds(&mut watch, &telemetry, None).is_empty());
        assert_eq!(kinds(&mut watch, &telemetry, Some(&job("Tomatoes", u32::MAX))), ["jobNew"]);
    }

    #[test]
    fn a_job_there_as_the_game_comes_isnt_news() {
        let at = Instant::now();
        let mut watch = Watch::default();
        let telemetry = Telemetry { game_time: 1_490, ..Telemetry::default() };
        let tomatoes = job("Tomatoes", 1_500);
        assert!(watch.tick(&telemetry, Some(&tomatoes), "truck", at).is_empty());
        assert!(watch.tick(&telemetry, Some(&tomatoes), "truck", at + SETTLE).is_empty());
    }
}
