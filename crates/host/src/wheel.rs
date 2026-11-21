//! The player's own phone as a steering wheel (owner 2026-11-19: "sem delay, como se fosse um volante mesmo"): the
//! wheel page (ui/src/wheel) sends what the phone's gyroscope and pedals say, already shaped by the player's
//! settings, on a WebSocket of its own (remote.rs, `/ws?…&wheel=1`): each value is written to the bridge as it
//! arrives (`HostBlock::wheel_*`), not behind the host's loop, and the plugin gives it to the game's steering,
//! throttle and brake (plugin/src/commands.rs). The phone gone quiet for [`QUIET`], or closed, and they're let go.
//!
//! The gyroscope is given to pages over HTTPS only, so the wheel page is served over HTTPS too ([`tls_config`]): a
//! certificate this PC makes once, signed by a little authority of its own (`/volante.cer`, which a phone may
//! install so the browser stops warning): the first time, the phone's browser warns, and the player goes on.

use std::net::IpAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use westplay_shared::BridgeMapping;

/// The phone quiet this long (it sends some 60 times a second), and the wheel is let go.
pub const QUIET: Duration = Duration::from_millis(250);

/// The phone wheel: who drives now, and when it last said.
pub struct Wheel {
    bridge: Arc<BridgeMapping>,
    /// The connection driving (its number), and when it last sent.
    driving: Mutex<Option<(u64, Instant)>>,
    connections: AtomicU64,
    /// Since when values are counted for the log, and how many came (one line every [`REPORT_EVERY`] while driven).
    counted: Mutex<(Instant, u32)>,
}

/// While the phone drives, the log says what it sends this often (to tell, from a test, whether values arrive).
const REPORT_EVERY: Duration = Duration::from_secs(5);

impl Wheel {
    /// The wheel, with a watchdog letting it go once the phone goes quiet.
    pub fn start(bridge: Arc<BridgeMapping>) -> Arc<Self> {
        let wheel = Arc::new(Self {
            bridge,
            driving: Mutex::new(None),
            connections: AtomicU64::new(0),
            counted: Mutex::new((Instant::now(), 0)),
        });
        wheel.write(None);
        let watched = Arc::downgrade(&wheel);
        let _ = std::thread::Builder::new().name("westplay-wheel-watchdog".into()).spawn(move || {
            while let Some(wheel) = watched.upgrade() {
                wheel.watch(Instant::now());
                drop(wheel);
                std::thread::sleep(Duration::from_millis(40));
            }
        });
        wheel
    }

    /// A number for a new connection, as [`Wheel::set`] and [`Wheel::gone`] take.
    pub fn connection(&self) -> u64 {
        self.connections.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// What connection `by` says: written to the bridge now. The last phone to say drives.
    pub fn set(&self, by: u64, values: [f32; 3], absolute: bool) {
        if let Ok(mut driving) = self.driving.lock() {
            *driving = Some((by, Instant::now()));
        }
        self.write(Some((values, absolute)));
        if let Ok(mut counted) = self.counted.lock() {
            counted.1 += 1;
            if counted.0.elapsed() >= REPORT_EVERY {
                let [steering, throttle, brake] = values;
                log!(
                    "[wheel] driving: {} values in {:.0} s, now steering {steering:+.2}, throttle {throttle:.2}, brake {brake:.2}",
                    counted.1,
                    counted.0.elapsed().as_secs_f32()
                );
                *counted = (Instant::now(), 0);
            }
        }
    }

    /// Connection `by` closed: the wheel let go if it drove.
    pub fn gone(&self, by: u64) {
        let Ok(mut driving) = self.driving.lock() else { return };
        if driving.is_some_and(|(driver, _)| driver == by) {
            *driving = None;
            drop(driving);
            self.write(None);
        }
    }

    /// Lets the wheel go if its driver went quiet.
    fn watch(&self, now: Instant) {
        let Ok(mut driving) = self.driving.lock() else { return };
        if driving.is_some_and(|(_, said)| now.duration_since(said) > QUIET) {
            *driving = None;
            drop(driving);
            log!("[wheel] the phone went quiet: let go");
            self.write(None);
        }
    }

    /// The values to the bridge, and whether the game's steering is absolute (`None`: let go, all at 0).
    fn write(&self, values: Option<([f32; 3], bool)>) {
        let host = &self.bridge.bridge().host;
        let [steering, throttle, brake] = values.map_or([0.0; 3], |(values, _)| values);
        host.wheel_steering.store(steering.to_bits(), Ordering::Release);
        host.wheel_throttle.store(throttle.to_bits(), Ordering::Release);
        host.wheel_brake.store(brake.to_bits(), Ordering::Release);
        host.wheel_seq.fetch_add(1, Ordering::AcqRel);
        // 1: the game's steering relative (the plugin steers toward the phone's angle), 2: absolute (plugin
        // commands.rs).
        host.wheel_active.store(values.map_or(0, |(_, absolute)| if absolute { 2 } else { 1 }), Ordering::Release);
    }
}

impl Wheel {
    /// The truck's state for the wheel page's buttons (`s <engine> <electric> <gear> <parking brake> <low beam>
    /// <high beam> <left blinker> <right blinker>`: 1 on, 0 off; the gear as the dashboard shows it, negative in
    /// reverse, 0 neutral), `None` before the game has said.
    pub fn state(&self) -> Option<String> {
        let t = self.bridge.bridge().plugin.telemetry.read()?;
        let on = |value: bool| u8::from(value);
        Some(format!(
            "s {} {} {} {} {} {} {} {}",
            on(t.engine_enabled),
            on(t.electric_enabled),
            t.displayed_gear,
            on(t.parking_brake),
            on(t.low_beam),
            on(t.high_beam),
            on(t.left_blinker),
            on(t.right_blinker),
        ))
    }
}

/// The wheel page's values (`w <steering> <throttle> <brake> [a]`: -1 to 1, 0 to 1, 0 to 1, and `a` when the
/// game's steering is absolute), clamped; `None` if it isn't one.
pub fn parse(text: &str) -> Option<([f32; 3], bool)> {
    let mut parts = text.strip_prefix("w ")?.split(' ');
    let mut value = || parts.next()?.parse::<f32>().ok().filter(|v| v.is_finite());
    let (steering, throttle, brake) = (value()?, value()?, value()?);
    let absolute = parts.next() == Some("a");
    Some(([steering.clamp(-1.0, 1.0), throttle.clamp(0.0, 1.0), brake.clamp(0.0, 1.0)], absolute))
}

/// The TLS the wheel page is served with: this PC's certificate for `address` (made once; again if the PC's
/// address changed, or it's getting old), and the authority that signed it ([`authority`]: kept, so a phone that
/// installed it goes on trusting the PC). `None` if it couldn't be made (logged).
pub fn tls_config(data_dir: &Path, address: IpAddr) -> Option<Arc<rustls::ServerConfig>> {
    match certificate(&data_dir.join("volante"), address) {
        Ok((chain, key)) => {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = rustls::ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .and_then(|builder| builder.with_no_client_auth().with_single_cert(chain, key));
            match config {
                Ok(config) => Some(Arc::new(config)),
                Err(error) => {
                    log!("[wheel] no HTTPS: {error}");
                    None
                }
            }
        }
        Err(error) => {
            log!("[wheel] no HTTPS: couldn't make the certificate: {error}");
            None
        }
    }
}

/// The authority's certificate (DER), for a phone to install (`/volante.cer`).
pub fn authority(data_dir: &Path) -> Option<Vec<u8>> {
    std::fs::read(data_dir.join("volante").join("ca.der")).ok()
}

/// The leaf certificate is made anew after this many days (Apple's phones refuse a server certificate of more
/// than 825).
const LEAF_DAYS: i64 = 800;
const LEAF_RENEW_DAYS: i64 = 700;

type Chain = (Vec<rustls::pki_types::CertificateDer<'static>>, rustls::pki_types::PrivateKeyDer<'static>);

fn certificate(dir: &Path, address: IpAddr) -> Result<Chain, String> {
    use rcgen::{
        BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType,
    };
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let e = |e: rcgen::Error| e.to_string();
    // The authority: its key kept; its certificate made again from the same name and key whenever a leaf is signed
    // (what a phone checks a leaf by), the first one kept for phones to install.
    let ca_key = match std::fs::read_to_string(dir.join("ca-key.pem")).ok().and_then(|pem| KeyPair::from_pem(&pem).ok())
    {
        Some(key) => key,
        None => {
            let key = KeyPair::generate().map_err(e)?;
            std::fs::write(dir.join("ca-key.pem"), key.serialize_pem()).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(dir.join("ca.der"));
            key
        }
    };
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).map_err(e)?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.distinguished_name.push(DnType::CommonName, "MaxDevMod Volante (este PC)");
    ca_params.distinguished_name.push(DnType::OrganizationName, "MaxDevMod");
    ca_params.key_usages =
        vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign, KeyUsagePurpose::DigitalSignature];
    ca_params.not_before = rcgen::date_time_ymd(2026, 1, 1);
    ca_params.not_after = rcgen::date_time_ymd(2046, 1, 1);
    let ca = ca_params.self_signed(&ca_key).map_err(e)?;
    let ca_der = match std::fs::read(dir.join("ca.der")) {
        Ok(der) => der,
        Err(_) => {
            std::fs::write(dir.join("ca.der"), ca.der().as_ref()).map_err(|e| e.to_string())?;
            ca.der().to_vec()
        }
    };

    let today = days_since_epoch();
    let made_for = std::fs::read_to_string(dir.join("leaf.txt")).unwrap_or_default();
    let kept = made_for.split_once(' ').and_then(|(ip, day)| Some((ip.to_owned(), day.trim().parse::<i64>().ok()?)));
    let fresh =
        kept.is_some_and(|(ip, day)| ip == address.to_string() && (0..LEAF_RENEW_DAYS).contains(&(today - day)));
    let leaf = if fresh {
        std::fs::read(dir.join("leaf.der"))
            .ok()
            .zip(std::fs::read_to_string(dir.join("leaf-key.pem")).ok().and_then(|pem| KeyPair::from_pem(&pem).ok()))
    } else {
        None
    };
    let (leaf_der, leaf_key) = match leaf {
        Some(leaf) => leaf,
        None => {
            let key = KeyPair::generate().map_err(e)?;
            let mut params = CertificateParams::new(vec!["localhost".to_owned()]).map_err(e)?;
            params.subject_alt_names.push(SanType::IpAddress(address));
            params.distinguished_name.push(DnType::CommonName, address.to_string());
            params.distinguished_name.push(DnType::OrganizationName, "MaxDevMod");
            params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
            params.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
            params.use_authority_key_identifier_extension = true;
            let (y, m, d) = civil(today - 1);
            params.not_before = rcgen::date_time_ymd(y, m, d);
            let (y, m, d) = civil(today + LEAF_DAYS);
            params.not_after = rcgen::date_time_ymd(y, m, d);
            let leaf = params.signed_by(&key, &ca, &ca_key).map_err(e)?;
            std::fs::write(dir.join("leaf.der"), leaf.der().as_ref()).map_err(|e| e.to_string())?;
            std::fs::write(dir.join("leaf-key.pem"), key.serialize_pem()).map_err(|e| e.to_string())?;
            std::fs::write(dir.join("leaf.txt"), format!("{address} {today}")).map_err(|e| e.to_string())?;
            log!("[wheel] made the HTTPS certificate for {address}");
            (leaf.der().to_vec(), key)
        }
    };
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
    Ok((vec![CertificateDer::from(leaf_der), CertificateDer::from(ca_der)], key))
}

fn days_since_epoch() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| (since.as_secs() / 86_400) as i64)
}

/// A day since 1970-01-01 as a year, month and day (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_wheel_pages_values_clamped() {
        assert_eq!(parse("w -0.250 0.800 0.000"), Some(([-0.25, 0.8, 0.0], false)));
        assert_eq!(parse("w -0.250 0.800 0.000 a"), Some(([-0.25, 0.8, 0.0], true)));
        assert_eq!(parse("w 3 -1 2"), Some(([1.0, 0.0, 1.0], false)));
        assert_eq!(parse("w 0.1 0.2"), None);
        assert_eq!(parse("w NaN 0 0"), None);
        assert_eq!(parse("w inf 0 0"), None);
        assert_eq!(parse("x 0 0 0"), None);
        assert_eq!(parse(r#"{"type":"ready"}"#), None);
    }

    #[test]
    fn tells_a_day_as_a_date() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_411), (2025, 11, 19));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn makes_its_certificate_once_and_again_for_a_new_address() {
        let dir = std::env::temp_dir().join(format!("westplay-wheel-cert-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first: IpAddr = "192.168.0.10".parse().unwrap();
        assert!(tls_config(&dir, first).is_some());
        let ca = authority(&dir).unwrap();
        let leaf = std::fs::read(dir.join("volante").join("leaf.der")).unwrap();
        assert!(tls_config(&dir, first).is_some());
        assert_eq!(std::fs::read(dir.join("volante").join("leaf.der")).unwrap(), leaf, "kept");
        assert!(tls_config(&dir, "10.0.0.5".parse().unwrap()).is_some());
        assert_ne!(std::fs::read(dir.join("volante").join("leaf.der")).unwrap(), leaf, "made anew");
        assert_eq!(authority(&dir).unwrap(), ca, "the authority kept");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
