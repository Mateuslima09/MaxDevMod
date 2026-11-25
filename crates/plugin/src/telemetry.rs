//! SCS telemetry → [`westplay_shared::Telemetry`] in the bridge.
//!
//! Channel callbacks update a local snapshot; at the end of every game frame the snapshot is published to
//! the shared memory. The truck's identity comes from the "truck" configuration event and is published
//! when it changes. All SDK callbacks arrive on the game's main thread.

use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use westplay_shared::{FixedStr, JobInfo, Telemetry, TruckInfo};

use crate::scs::{
    ChannelCallback, ConfigurationEvent, EventCallback, SCS_RESULT_OK, SCS_TELEMETRY_EVENT_CONFIGURATION,
    SCS_TELEMETRY_EVENT_FRAME_END, SCS_TELEMETRY_EVENT_GAMEPLAY, SCS_TELEMETRY_EVENT_PAUSED,
    SCS_TELEMETRY_EVENT_STARTED, SCS_U32_NIL, SCS_VALUE_TYPE_BOOL, SCS_VALUE_TYPE_DPLACEMENT, SCS_VALUE_TYPE_FLOAT,
    SCS_VALUE_TYPE_FPLACEMENT, SCS_VALUE_TYPE_S32, SCS_VALUE_TYPE_U32, ScsValue, TelemetryInitParamsV100,
};
use crate::{bridge, log};

/// Whether the game is paused (its menu, its map: they take the mouse), as its events last said; paused until
/// it says it runs.
static PAUSED: AtomicBool = AtomicBool::new(true);

/// Whether the game is paused (any thread).
pub fn paused() -> bool {
    PAUSED.load(Ordering::Acquire)
}

/// What the walking test (`character`) needs of the truck: where it is and how it's turned (metres; heading
/// in turns), its speed, its parking brake, whether this is a multiplayer session (the SDK's time offset),
/// and where the driver's head is in it (the cab's position plus the head's, in the truck's own space).
pub struct Truck {
    pub position: [f64; 3],
    pub heading: f32,
    /// Its pitch and roll (turns), as the SDK says.
    pub pitch: f32,
    pub roll: f32,
    pub speed: f32,
    pub parking_brake: bool,
    pub multiplayer: bool,
    pub head: [f32; 3],
}

/// The truck as the SDK last said (any thread).
pub fn truck() -> Truck {
    let s = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    let [hx, hy, hz] = *HEAD_DEFAULT.lock().unwrap_or_else(|e| e.into_inner());
    let [cx, cy, cz] = *CABIN.lock().unwrap_or_else(|e| e.into_inner());
    Truck {
        position: [s.x, s.y, s.z],
        heading: s.heading,
        pitch: s.pitch,
        roll: s.roll,
        speed: s.speed,
        parking_brake: s.parking_brake,
        multiplayer: s.multiplayer_time_offset != 0,
        head: [cx + hx + s.head_x, cy + hy + s.head_y, cz + hz + s.head_z],
    }
}

/// Where the driver's head is by default in the truck (its own space: the cab's position plus the head's default,
/// not where it's looking now): the floating screen's own place is worked out from it (`float_screen`).
pub fn head_default() -> [f32; 3] {
    let [hx, hy, hz] = *HEAD_DEFAULT.lock().unwrap_or_else(|e| e.into_inner());
    let [cx, cy, cz] = *CABIN.lock().unwrap_or_else(|e| e.into_inner());
    [cx + hx, cy + hy, cz + hz]
}

/// The trailer the SDK tells of (the job's, or the player's own), for the cargo's papers and coupling by hand
/// (`character::freight`): whether it's coupled, where it is and how it's turned (metres; heading in turns; not
/// `placed` until the SDK has said), and where its coupling pin is in it (the "trailer" configuration's
/// `hook.position`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Trailer {
    pub connected: bool,
    pub placed: bool,
    pub position: [f64; 3],
    pub heading: f32,
    /// Its pitch and roll (turns), as the SDK says: the reversing camera on its back tilts with it.
    pub pitch: f32,
    pub roll: f32,
    pub hook: Option<[f32; 3]>,
}

static TRAILER: Mutex<Trailer> =
    Mutex::new(Trailer { connected: false, placed: false, position: [0.0; 3], heading: 0.0, pitch: 0.0, roll: 0.0, hook: None });

/// Additional coupled trailers (`trailer.1`..=`trailer.3`) for multi-trailer combinations (bitrem / rodotrem / triples),
/// tracked with their own world placements and SDK wheel hub coordinates so brake smoke emits from every trailer's wheels.
pub const EXTRA_TRAILERS_COUNT: usize = 3;

#[derive(Clone, Copy, Debug)]
pub struct ExtraTrailer {
    pub connected: bool,
    pub placed: bool,
    pub position: [f64; 3],
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
    pub wheel_count: usize,
    pub wheel_at: [[f32; 3]; crate::reversing::MOST_WHEELS],
}

impl ExtraTrailer {
    const fn empty() -> Self {
        Self {
            connected: false,
            placed: false,
            position: [0.0; 3],
            heading: 0.0,
            pitch: 0.0,
            roll: 0.0,
            wheel_count: 0,
            wheel_at: [[0.0; 3]; crate::reversing::MOST_WHEELS],
        }
    }
}

static EXTRA_TRAILERS: Mutex<[ExtraTrailer; EXTRA_TRAILERS_COUNT]> =
    Mutex::new([ExtraTrailer::empty(); EXTRA_TRAILERS_COUNT]);

/// The trailer as the SDK last said (any thread).
pub fn trailer() -> Trailer {
    *TRAILER.lock().unwrap_or_else(|e| e.into_inner())
}

/// Additional trailers (`trailer.1`..=`trailer.3`) as the SDK last said (any thread).
pub fn extra_trailers() -> [ExtraTrailer; EXTRA_TRAILERS_COUNT] {
    *EXTRA_TRAILERS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where the truck's fifth wheel is in it (the "truck" configuration's `hook.position`), if the SDK said.
static TRUCK_HOOK: Mutex<Option<[f32; 3]>> = Mutex::new(None);

pub fn truck_hook() -> Option<[f32; 3]> {
    *TRUCK_HOOK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The fuel in the truck's tank (litres), as the SDK last said (any thread).
pub fn fuel() -> f32 {
    SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).fuel
}

/// Whether the truck's engine runs.
pub fn engine_on() -> bool {
    SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).engine_enabled
}

/// The tank's size (litres: the "truck" configuration's `fuel.capacity`), 0 until the SDK has said.
static FUEL_CAPACITY: Mutex<f32> = Mutex::new(0.0);

pub fn fuel_capacity() -> f32 {
    *FUEL_CAPACITY.lock().unwrap_or_else(|e| e.into_inner())
}

/// The player's job as the SDK last configured it (`active` false: none).
static JOB: Mutex<Option<JobInfo>> = Mutex::new(None);

pub fn job() -> Option<JobInfo> {
    (*JOB.lock().unwrap_or_else(|e| e.into_inner())).filter(|job| job.active)
}

/// How far the game's GPS route still goes (metres; 0: no route).
pub fn navigation_distance() -> f32 {
    SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).navigation_distance
}

static SNAPSHOT: Mutex<Telemetry> = Mutex::new(Telemetry {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    heading: 0.0,
    pitch: 0.0,
    roll: 0.0,
    speed: 0.0,
    engine_rpm: 0.0,
    gear: 0,
    displayed_gear: 0,
    cruise_control_speed: 0.0,
    fuel: 0.0,
    fuel_range: 0.0,
    navigation_distance: 0.0,
    navigation_time: 0.0,
    navigation_speed_limit: 0.0,
    game_time: 0,
    local_scale: 0.0,
    dashboard_backlight: 0.0,
    electric_enabled: false,
    engine_enabled: false,
    parking_brake: false,
    low_beam: false,
    high_beam: false,
    left_blinker: false,
    right_blinker: false,
    paused: true,
    head_x: 0.0,
    head_y: 0.0,
    head_z: 0.0,
    head_heading: 0.0,
    head_pitch: 0.0,
    camera_known: false,
    camera_x: 0.0,
    camera_y: 0.0,
    camera_z: 0.0,
    cabin_x: 0.0,
    cabin_y: 0.0,
    cabin_z: 0.0,
    steering: 0.0,
    brake_temperature: 0.0,
    input_brake: 0.0,
    multiplayer_time_offset: 0,
    wheel_steering: 0.0,
    front_axle_z: 0.0,
    rear_axle_z: 0.0,
    hook_z: 0.0,
    rear_z: 0.0,
    half_width: 0.0,
    trailer_connected: false,
    trailer_x: 0.0,
    trailer_y: 0.0,
    trailer_z: 0.0,
    trailer_heading: 0.0,
    trailer_hook_z: 0.0,
    trailer_axle_z: 0.0,
    trailer_rear_z: 0.0,
    trailer_half_width: 0.0,
    ray_sensors: [-1.0; 4],
    rays: 0,
    mirror: false,
    suspension: [-1.0; 3],
    brake: 0.0,
    camera_ready: false,
    wear: [0.0; 5],
    fuel_warning: false,
    rest_stop: 0,
    rest_known: false,
    camera_back: 0.0,
    camera_up: 0.0,
    camera_tilt: 0.0,
    camera_turn: 0.0,
    camera_wide: 0.0,
    float_on: false,
    float_seen: false,
    float_pose: [0.0; 6],
    camera_side: 0.0,
    camera_roll: 0.0,
});

/// Where the driver's head is by default in the truck's cab (the "truck" configuration's `head.position`).
/// The snapshot holds the head's offset from it (the `truck.head.offset` channel); they're added together as
/// the snapshot is published.
static HEAD_DEFAULT: Mutex<[f32; 3]> = Mutex::new([0.0; 3]);

/// Where the cab turns in the truck (the "truck" configuration's `cabin.position`), published with every
/// snapshot.
static CABIN: Mutex<[f32; 3]> = Mutex::new([0.0; 3]);

#[derive(Clone, Copy)]
enum Field {
    Placement,
    Speed,
    EngineRpm,
    Gear,
    DisplayedGear,
    CruiseControl,
    Fuel,
    FuelRange,
    NavigationDistance,
    NavigationTime,
    NavigationSpeedLimit,
    GameTime,
    LocalScale,
    DashboardBacklight,
    ElectricEnabled,
    EngineEnabled,
    ParkingBrake,
    LowBeam,
    HighBeam,
    LeftBlinker,
    RightBlinker,
    HeadOffset,
    Steering,
    Brake,
    BrakeTemperature,
    InputBrake,
    MultiplayerTimeOffset,
    TrailerConnected,
    TrailerPlacement,
    /// One of `Telemetry::wear`.
    Wear(usize),
    FuelWarning,
    RestStop,
}

/// The context `truck.wheel.steering` is registered with for its wheel 0 (its index added): apart from the
/// [`CHANNELS`]' indices.
const WHEEL_CONTEXT: usize = 0x1000;
/// The contexts `truck.wheel.suspension.deflection` and `trailer.wheel.suspension.deflection` are registered with
/// for their wheel 0 (the phone's Suspensão app).
const SUSPENSION_CONTEXT: usize = 0x2000;
const TRAILER_SUSPENSION_CONTEXT: usize = 0x3000;
/// Contexts for additional trailers (`trailer.1`..=`trailer.3`) connected and world placement channels.
const EXTRA_TRAILER_CONNECTED_CONTEXT: usize = 0x4000;
const EXTRA_TRAILER_PLACEMENT_CONTEXT: usize = 0x4100;

const CHANNELS: &[(&str, u32, Field)] = &[
    ("truck.world.placement", SCS_VALUE_TYPE_DPLACEMENT, Field::Placement),
    ("truck.speed", SCS_VALUE_TYPE_FLOAT, Field::Speed),
    ("truck.engine.rpm", SCS_VALUE_TYPE_FLOAT, Field::EngineRpm),
    ("truck.engine.gear", SCS_VALUE_TYPE_S32, Field::Gear),
    ("truck.displayed.gear", SCS_VALUE_TYPE_S32, Field::DisplayedGear),
    ("truck.cruise_control", SCS_VALUE_TYPE_FLOAT, Field::CruiseControl),
    ("truck.fuel.amount", SCS_VALUE_TYPE_FLOAT, Field::Fuel),
    ("truck.fuel.range", SCS_VALUE_TYPE_FLOAT, Field::FuelRange),
    ("truck.navigation.distance", SCS_VALUE_TYPE_FLOAT, Field::NavigationDistance),
    ("truck.navigation.time", SCS_VALUE_TYPE_FLOAT, Field::NavigationTime),
    ("truck.navigation.speed.limit", SCS_VALUE_TYPE_FLOAT, Field::NavigationSpeedLimit),
    ("game.time", SCS_VALUE_TYPE_U32, Field::GameTime),
    ("local.scale", SCS_VALUE_TYPE_FLOAT, Field::LocalScale),
    ("truck.dashboard.backlight", SCS_VALUE_TYPE_FLOAT, Field::DashboardBacklight),
    ("truck.electric.enabled", SCS_VALUE_TYPE_BOOL, Field::ElectricEnabled),
    ("truck.engine.enabled", SCS_VALUE_TYPE_BOOL, Field::EngineEnabled),
    ("truck.brake.parking", SCS_VALUE_TYPE_BOOL, Field::ParkingBrake),
    ("truck.light.beam.low", SCS_VALUE_TYPE_BOOL, Field::LowBeam),
    ("truck.light.beam.high", SCS_VALUE_TYPE_BOOL, Field::HighBeam),
    ("truck.light.lblinker", SCS_VALUE_TYPE_BOOL, Field::LeftBlinker),
    ("truck.light.rblinker", SCS_VALUE_TYPE_BOOL, Field::RightBlinker),
    ("truck.head.offset", SCS_VALUE_TYPE_FPLACEMENT, Field::HeadOffset),
    ("truck.effective.steering", SCS_VALUE_TYPE_FLOAT, Field::Steering),
    ("truck.effective.brake", SCS_VALUE_TYPE_FLOAT, Field::Brake),
    // Brake fade (owner 2026-11-21, docs/fadiga-freio/PLANO.md): the game's brake temperature and the pedal before it.
    ("truck.brake.temperature", SCS_VALUE_TYPE_FLOAT, Field::BrakeTemperature),
    ("truck.input.brake", SCS_VALUE_TYPE_FLOAT, Field::InputBrake),
    ("multiplayer.time.offset", SCS_VALUE_TYPE_S32, Field::MultiplayerTimeOffset),
    ("trailer.connected", SCS_VALUE_TYPE_BOOL, Field::TrailerConnected),
    ("trailer.world.placement", SCS_VALUE_TYPE_DPLACEMENT, Field::TrailerPlacement),
    // The phone's notifications (owner 2026-11-20): the truck's damage, its fuel warning, the driver's rest.
    ("truck.wear.engine", SCS_VALUE_TYPE_FLOAT, Field::Wear(0)),
    ("truck.wear.transmission", SCS_VALUE_TYPE_FLOAT, Field::Wear(1)),
    ("truck.wear.cabin", SCS_VALUE_TYPE_FLOAT, Field::Wear(2)),
    ("truck.wear.chassis", SCS_VALUE_TYPE_FLOAT, Field::Wear(3)),
    ("truck.wear.wheels", SCS_VALUE_TYPE_FLOAT, Field::Wear(4)),
    ("truck.fuel.warning", SCS_VALUE_TYPE_BOOL, Field::FuelWarning),
    ("rest.stop", SCS_VALUE_TYPE_S32, Field::RestStop),
];

/// Registers the telemetry channels and events. Called from `scs_telemetry_init`.
///
/// # Safety
/// During `scs_telemetry_init`, with the parameters the game passed it.
pub unsafe fn register(params: &TelemetryInitParamsV100) {
    // SAFETY: the game's two registration functions, with the SDK's signatures (`EventRegisterFn`,
    // `ChannelRegisterFn`); the callbacks are ours, of the signatures the SDK calls, and the channel names are
    // copied by the SDK as they're registered.
    unsafe {
        let register_event: EventRegisterFn = std::mem::transmute(params.register_for_event);
        for event in [
            SCS_TELEMETRY_EVENT_FRAME_END,
            SCS_TELEMETRY_EVENT_PAUSED,
            SCS_TELEMETRY_EVENT_STARTED,
            SCS_TELEMETRY_EVENT_CONFIGURATION,
            SCS_TELEMETRY_EVENT_GAMEPLAY,
        ] {
            let result = register_event(event, on_event, std::ptr::null_mut());
            if result != SCS_RESULT_OK {
                log!("telemetry: event {event} registration failed ({result})");
            }
        }

        let register_channel: ChannelRegisterFn = std::mem::transmute(params.register_for_channel);
        let mut registered = 0;
        for (index, &(name, value_type, _)) in CHANNELS.iter().enumerate() {
            let Ok(c_name) = CString::new(name) else { continue };
            // The SDK copies the name during registration.
            let result =
                register_channel(c_name.as_ptr(), SCS_U32_NIL, value_type, 0, on_channel, index as *mut c_void);
            if result == SCS_RESULT_OK {
                registered += 1;
            } else {
                log!("telemetry: channel {name} registration failed ({result})");
            }
        }
        log!("telemetry: {registered}/{} channels registered", CHANNELS.len());
        // How far each wheel is turned, for the reversing camera's lines (only the steered ones are read).
        let Ok(c_name) = CString::new("truck.wheel.steering") else { return };
        let wheels = (0..crate::reversing::MOST_WHEELS)
            .filter(|&wheel| {
                let context = (WHEEL_CONTEXT + wheel) as *mut c_void;
                register_channel(c_name.as_ptr(), wheel as u32, SCS_VALUE_TYPE_FLOAT, 0, on_channel, context)
                    == SCS_RESULT_OK
            })
            .count();
        log!("telemetry: the steering of {wheels} wheels registered");
        // How far each wheel's spring is pressed in, the truck's and the trailer's (the phone's Suspensão app).
        for (channel, base) in [
            ("truck.wheel.suspension.deflection", SUSPENSION_CONTEXT),
            ("trailer.wheel.suspension.deflection", TRAILER_SUSPENSION_CONTEXT),
        ] {
            let Ok(c_name) = CString::new(channel) else { continue };
            let wheels = (0..crate::reversing::MOST_WHEELS)
                .filter(|&wheel| {
                    let context = (base + wheel) as *mut c_void;
                    register_channel(c_name.as_ptr(), wheel as u32, SCS_VALUE_TYPE_FLOAT, 0, on_channel, context)
                        == SCS_RESULT_OK
                })
                .count();
            log!("telemetry: {channel} of {wheels} wheels registered");
        }
        // Additional coupled trailers (`trailer.1`..=`trailer.3`) for multi-trailer brake smoke.
        let mut extra_registered = 0usize;
        for i in 0..EXTRA_TRAILERS_COUNT {
            let num = i + 1;
            let Ok(conn_name) = CString::new(format!("trailer.{num}.connected")) else { continue };
            let Ok(place_name) = CString::new(format!("trailer.{num}.world.placement")) else { continue };
            let c_ok = register_channel(
                conn_name.as_ptr(),
                SCS_U32_NIL,
                SCS_VALUE_TYPE_BOOL,
                0,
                on_channel,
                (EXTRA_TRAILER_CONNECTED_CONTEXT + i) as *mut c_void,
            ) == SCS_RESULT_OK;
            let p_ok = register_channel(
                place_name.as_ptr(),
                SCS_U32_NIL,
                SCS_VALUE_TYPE_DPLACEMENT,
                0,
                on_channel,
                (EXTRA_TRAILER_PLACEMENT_CONTEXT + i) as *mut c_void,
            ) == SCS_RESULT_OK;
            if c_ok && p_ok {
                extra_registered += 1;
            }
        }
        if extra_registered > 0 {
            log!("telemetry: {extra_registered} extra trailers registered");
        }
    }
}

type EventRegisterFn = unsafe extern "system" fn(u32, EventCallback, *mut c_void) -> i32;
type ChannelRegisterFn = unsafe extern "system" fn(*const c_char, u32, u32, u32, ChannelCallback, *mut c_void) -> i32;

/// # Safety
/// Called by the game, as the SDK says: on its main thread, `info` the event's (or null), alive through
/// the call.
unsafe extern "system" fn on_event(event: u32, info: *const c_void, _context: *mut c_void) {
    // SAFETY: as the SDK calls it (below), so `on_game_event` is called.
    unsafe {
        crate::hooks::contained("a telemetry event", (), || on_game_event(event, info));
    }
}

/// # Safety
/// As [`on_event`].
unsafe fn on_game_event(event: u32, info: *const c_void) {
    // SAFETY: a configuration event's `info` is the SDK's `ConfigurationEvent`, alive through the call; the
    // navigator is served on the game's main thread, where this is (`serve`'s contract).
    unsafe {
        // The money the game moved (money.rs): a gameplay event is laid out as a configuration.
        if event == SCS_TELEMETRY_EVENT_GAMEPLAY {
            if let Some(gameplay) = (info as *const ConfigurationEvent).as_ref() {
                crate::money::on_gameplay(gameplay);
            }
            return;
        }
        if event == SCS_TELEMETRY_EVENT_CONFIGURATION {
            if let Some(configuration) = (info as *const ConfigurationEvent).as_ref() {
                on_configuration(configuration);
            }
            return;
        }
        let mut snapshot = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
        match event {
            // Logged as they come: the screen's use and the music stop and go on with them.
            SCS_TELEMETRY_EVENT_PAUSED => {
                if !snapshot.paused {
                    log!("the game paused");
                }
                snapshot.paused = true;
                PAUSED.store(true, Ordering::Release);
            }
            SCS_TELEMETRY_EVENT_STARTED => {
                if snapshot.paused {
                    log!("the game runs again");
                }
                snapshot.paused = false;
                PAUSED.store(false, Ordering::Release);
            }
            SCS_TELEMETRY_EVENT_FRAME_END => {
                if let Some(bridge) = bridge::get() {
                    let [x, y, z] = *HEAD_DEFAULT.lock().unwrap_or_else(|e| e.into_inner());
                    let [cabin_x, cabin_y, cabin_z] = *CABIN.lock().unwrap_or_else(|e| e.into_inner());
                    let vehicles = crate::reversing::VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
                    let (truck_layout, trailer_layout) = (vehicles.truck, vehicles.trailer);
                    let wheel_steering = vehicles.wheel_steering();
                    // The trailer for the reversing camera: where it is once coupled (kept apart, `TRAILER`).
                    let trailer = trailer();
                    let coupled = trailer.connected && trailer.placed;
                    let suspension = vehicles.suspension(coupled);
                    drop(vehicles);
                    let [trailer_x, trailer_y, trailer_z] = if coupled { trailer.position } else { [0.0; 3] };
                    let trailer_heading = if coupled { trailer.heading } else { 0.0 };
                    let camera = crate::camera::position();
                    let [camera_x, camera_y, camera_z] = camera.unwrap_or_default();
                    let published = Telemetry {
                        head_x: snapshot.head_x + x,
                        head_y: snapshot.head_y + y,
                        head_z: snapshot.head_z + z,
                        camera_known: camera.is_some(),
                        camera_x,
                        camera_y,
                        camera_z,
                        cabin_x,
                        cabin_y,
                        cabin_z,
                        wheel_steering,
                        front_axle_z: truck_layout.front_axle_z,
                        rear_axle_z: truck_layout.rear_axle_z,
                        hook_z: truck_layout.hook_z,
                        rear_z: truck_layout.rear_z,
                        half_width: truck_layout.half_width,
                        trailer_connected: coupled,
                        trailer_x,
                        trailer_y,
                        trailer_z,
                        trailer_heading,
                        trailer_hook_z: trailer_layout.hook_z,
                        trailer_axle_z: trailer_layout.rear_axle_z,
                        trailer_rear_z: trailer_layout.rear_z,
                        trailer_half_width: trailer_layout.half_width,
                        suspension,
                        ..*snapshot
                    };
                    // The parking sensors through the game's physics: this frame's rays asked for, the last read.
                    let mut published = published;
                    crate::ray::frame(&mut published);
                    published.mirror = crate::mirror::seen();
                    published.camera_ready = crate::rear_camera::ready();
                    [
                        published.camera_back,
                        published.camera_up,
                        published.camera_tilt,
                        published.camera_turn,
                        published.camera_wide,
                        published.camera_side,
                        published.camera_roll,
                    ] = crate::rear_camera::nudged(&published);
                    (published.float_on, published.float_seen, published.float_pose) = crate::float_screen::published(&published);
                    bridge.plugin.telemetry.write(&published);
                }
                let (speed, paused) = (snapshot.speed, snapshot.paused);
                drop(snapshot);
                // Whether the player's keys type into WestPlay's field: they stop the moment the truck moves.
                crate::typing::update(speed, paused);
                // The end of a frame is on the game's main thread, where its navigator may be changed.
                crate::navigation::serve();
                // In-memory Job Market (Bolsa de Fretes): snapshot or accept offers on the game's main thread.
                crate::job_market::serve();
                // In-memory Nupay Bank & Loans: live balance mirror and instant take/repay loan on the main thread.
                crate::bank::serve();
                // In-memory Company / Fleet & Garage Manager: trucks, trailers, and garage upgrades on the main thread.
                crate::fleet::serve();
                // The walking test: on the game's main thread too (it calls the game's camera).
                crate::character::on_frame_end();
            }
            _ => {}
        }
    }
}

/// Publishes what the game (re)configures: the truck's identity and the player's job. An empty attribute set
/// means the player has none right now.
///
/// # Safety
/// `configuration` is the game's, alive through the call (an event's `info`).
unsafe fn on_configuration(configuration: &ConfigurationEvent) {
    // SAFETY: the SDK's strings, nul-terminated and alive through the call; checked for none first.
    unsafe {
        if configuration.id.is_null() {
            return;
        }
        match CStr::from_ptr(configuration.id).to_bytes() {
            b"truck" => on_truck(configuration),
            b"job" => on_job(configuration),
            // The first trailer (a game from before trailers were numbered says it without its number).
            b"trailer" | b"trailer.0" => {
                let hook = configuration
                    .attributes()
                    .find(|attribute| CStr::from_ptr(attribute.name).to_bytes() == b"hook.position")
                    .and_then(|attribute| attribute.value.fvector());
                let mut trailer = TRAILER.lock().unwrap_or_else(|e| e.into_inner());
                if trailer.hook != hook {
                    log!("trailer: its coupling pin at {hook:?}");
                    trailer.hook = hook;
                }
                drop(trailer);
                // Its wheels and length, for the reversing camera.
                on_trailer(configuration);
            }
            b"trailer.1" => on_extra_trailer(0, configuration),
            b"trailer.2" => on_extra_trailer(1, configuration),
            b"trailer.3" => on_extra_trailer(2, configuration),
            _ => {}
        }
    }
}

/// # Safety
/// As [`on_configuration`].
unsafe fn on_truck(configuration: &ConfigurationEvent) {
    // SAFETY: the attributes' names are the SDK's strings, nul-terminated and alive through the call.
    unsafe {
        let mut truck = TruckInfo::default();
        let mut head = [0.0; 3];
        let mut cabin = [0.0; 3];
        let mut hook = None;
        let mut fuel_capacity = 0.0;
        let mut configured = crate::reversing::Configured::default();
        for attribute in configuration.attributes() {
            configured.take(attribute);
            match CStr::from_ptr(attribute.name).to_bytes() {
                b"fuel.capacity" => {
                    fuel_capacity = attribute.value.float();
                    continue;
                }
                b"hook.position" => {
                    hook = attribute.value.fvector();
                    continue;
                }
                b"head.position" => {
                    head = attribute.value.fvector().unwrap_or_default();
                    continue;
                }
                b"cabin.position" => {
                    cabin = attribute.value.fvector().unwrap_or_default();
                    continue;
                }
                _ => {}
            }
            let Some(value) = attribute.value.string() else { continue };
            match CStr::from_ptr(attribute.name).to_bytes() {
                b"brand_id" => truck.brand_id = FixedStr::new(&value),
                b"brand" => truck.brand = FixedStr::new(&value),
                b"id" => truck.id = FixedStr::new(&value),
                b"name" => truck.name = FixedStr::new(&value),
                b"license.plate" => truck.plate = FixedStr::new(&value),
                b"license.plate.country.id" => truck.plate_country = FixedStr::new(&value),
                _ => {}
            }
        }
        truck.head = head;
        log!(
            "truck: {} {} ({}, {}), plate {:?} ({}), head at {head:?}, cab turning at {cabin:?}, fifth wheel at {hook:?}",
            truck.brand.as_str(),
            truck.name.as_str(),
            truck.brand_id.as_str(),
            truck.id.as_str(),
            truck.plate.as_str(),
            truck.plate_country.as_str()
        );
        *HEAD_DEFAULT.lock().unwrap_or_else(|e| e.into_inner()) = head;
        *CABIN.lock().unwrap_or_else(|e| e.into_inner()) = cabin;
        *TRUCK_HOOK.lock().unwrap_or_else(|e| e.into_inner()) = hook;
        *FUEL_CAPACITY.lock().unwrap_or_else(|e| e.into_inner()) = fuel_capacity;
        crate::reversing::truck_configured(&configured);
        if let Some(bridge) = bridge::get() {
            bridge.plugin.truck.write(&truck);
        }
        crate::screens::truck_changed(truck.id.as_str());
    }
}

/// The first trailer's layout, for the reversing camera (no attributes: no trailer).
///
/// # Safety
/// As [`on_configuration`].
unsafe fn on_trailer(configuration: &ConfigurationEvent) {
    // SAFETY: the attributes are the SDK's, alive through the call (this function's contract).
    unsafe {
        let mut configured = crate::reversing::Configured::default();
        for attribute in configuration.attributes() {
            configured.take(attribute);
        }
        crate::reversing::trailer_configured(&configured);
    }
}

/// Additional coupled trailer's wheel hub positions (`trailer.1`..=`trailer.3`), for multi-trailer brake smoke.
///
/// # Safety
/// As [`on_configuration`].
unsafe fn on_extra_trailer(slot: usize, configuration: &ConfigurationEvent) {
    // SAFETY: the attributes are the SDK's, alive through the call (this function's contract).
    unsafe {
        let mut configured = crate::reversing::Configured::default();
        for attribute in configuration.attributes() {
            configured.take(attribute);
        }
        let mut extras = EXTRA_TRAILERS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = extras.get_mut(slot) {
            t.wheel_count = configured.count;
            for at in 0..crate::reversing::MOST_WHEELS {
                t.wheel_at[at] = if at < configured.count { configured.wheels[at].at } else { [0.0; 3] };
            }
        }
    }
}

/// # Safety
/// As [`on_configuration`].
unsafe fn on_job(configuration: &ConfigurationEvent) {
    // SAFETY: the attributes' names are the SDK's strings, nul-terminated and alive through the call.
    unsafe {
        let mut job = JobInfo::default();
        for attribute in configuration.attributes() {
            job.active = true;
            let value = &attribute.value;
            let text = || FixedStr::new(&value.string().unwrap_or_default());
            match CStr::from_ptr(attribute.name).to_bytes() {
                b"cargo" => job.cargo = text(),
                b"cargo.mass" => job.cargo_mass = value.float(),
                b"cargo.loaded" => job.cargo_loaded = value.bool(),
                b"is.special.job" => job.special = value.bool(),
                b"source.city" => job.source_city = text(),
                b"source.company" => job.source_company = text(),
                b"destination.city" => job.destination_city = text(),
                b"destination.company" => job.destination_company = text(),
                b"delivery.time" => job.delivery_time = value.u32(),
                b"planned_distance.km" => job.planned_distance_km = value.u32(),
                b"income" => job.income = value.u64(),
                _ => {}
            }
        }
        log!(
            "job: {} ({} kg) from {}, {} to {}, {}",
            job.cargo.as_str(),
            job.cargo_mass,
            job.source_company.as_str(),
            job.source_city.as_str(),
            job.destination_company.as_str(),
            job.destination_city.as_str()
        );
        *JOB.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
        if let Some(bridge) = bridge::get() {
            bridge.plugin.job.write(&job);
        }
    }
}

/// # Safety
/// Called by the game, as the SDK says: `value` the channel's (or null, no value), alive through the
/// call; `context` the index it was registered with.
unsafe extern "system" fn on_channel(_name: *const c_char, _index: u32, value: *const ScsValue, context: *mut c_void) {
    // SAFETY: as the SDK calls it (below), so `on_channel_value` is called.
    unsafe {
        crate::hooks::contained("a telemetry channel", (), || on_channel_value(value, context));
    }
}

/// # Safety
/// As [`on_channel`].
unsafe fn on_channel_value(value: *const ScsValue, context: *mut c_void) {
    // SAFETY: `value` is the SDK's, of the type the channel was registered with (each `Field` reads its own),
    // or null.
    unsafe {
        // A wheel's suspension, the truck's or the trailer's (`register`): metres pressed in.
        let at = context as usize;
        if (EXTRA_TRAILER_CONNECTED_CONTEXT..EXTRA_TRAILER_CONNECTED_CONTEXT + EXTRA_TRAILERS_COUNT).contains(&at) {
            let slot = at - EXTRA_TRAILER_CONNECTED_CONTEXT;
            let mut extras = EXTRA_TRAILERS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(t) = extras.get_mut(slot) {
                t.connected = value.as_ref().is_some_and(|v| v.bool());
            }
            return;
        }
        if (EXTRA_TRAILER_PLACEMENT_CONTEXT..EXTRA_TRAILER_PLACEMENT_CONTEXT + EXTRA_TRAILERS_COUNT).contains(&at) {
            let slot = at - EXTRA_TRAILER_PLACEMENT_CONTEXT;
            let mut extras = EXTRA_TRAILERS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(t) = extras.get_mut(slot) {
                if let Some(v) = value.as_ref() {
                    let p = v.dplacement();
                    t.position = [p.x, p.y, p.z];
                    t.heading = p.heading;
                    (t.pitch, t.roll) = (p.pitch, p.roll);
                    t.placed = p.x != 0.0 || p.z != 0.0;
                } else {
                    t.placed = false;
                }
            }
            return;
        }
        if (SUSPENSION_CONTEXT..TRAILER_SUSPENSION_CONTEXT + crate::reversing::MOST_WHEELS).contains(&at) {
            let deflection = value.as_ref().map_or(0.0, ScsValue::float);
            let mut vehicles = crate::reversing::VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
            if at >= TRAILER_SUSPENSION_CONTEXT {
                if let Some(slot) = vehicles.trailer_deflection.get_mut(at - TRAILER_SUSPENSION_CONTEXT) {
                    *slot = deflection;
                }
            } else if let Some(slot) = vehicles.deflection.get_mut(at - SUSPENSION_CONTEXT) {
                *slot = deflection;
            }
            return;
        }
        // A wheel's steering (`register`): its turns, none without a value.
        if let Some(wheel) = (context as usize).checked_sub(WHEEL_CONTEXT) {
            let mut vehicles = crate::reversing::VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(steering) = vehicles.steering.get_mut(wheel) {
                *steering = value.as_ref().map_or(0.0, ScsValue::float);
            }
            return;
        }
        let Some(&(_, _, field)) = CHANNELS.get(context as usize) else { return };
        // The trailer's are kept apart (`TRAILER`): none of the pages'.
        if matches!(field, Field::TrailerConnected | Field::TrailerPlacement) {
            let mut trailer = TRAILER.lock().unwrap_or_else(|e| e.into_inner());
            match (field, value.as_ref()) {
                (Field::TrailerConnected, value) => trailer.connected = value.is_some_and(|value| value.bool()),
                (_, Some(value)) => {
                    let p = value.dplacement();
                    trailer.position = [p.x, p.y, p.z];
                    trailer.heading = p.heading;
                    (trailer.pitch, trailer.roll) = (p.pitch, p.roll);
                    // Before a trailer is there, the SDK says the world's origin.
                    trailer.placed = p.x != 0.0 || p.z != 0.0;
                }
                (_, None) => trailer.placed = false,
            }
            return;
        }
        let mut s = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
        // A null value means the channel currently has no value (e.g. no truck); reset to the default.
        let Some(value) = value.as_ref() else {
            reset(&mut s, field);
            return;
        };
        match field {
            Field::Placement => {
                let p = value.dplacement();
                (s.x, s.y, s.z) = (p.x, p.y, p.z);
                (s.heading, s.pitch, s.roll) = (p.heading, p.pitch, p.roll);
            }
            Field::Speed => s.speed = value.float(),
            Field::EngineRpm => s.engine_rpm = value.float(),
            Field::Gear => s.gear = value.s32(),
            Field::DisplayedGear => s.displayed_gear = value.s32(),
            Field::CruiseControl => s.cruise_control_speed = value.float(),
            Field::Fuel => s.fuel = value.float(),
            Field::FuelRange => s.fuel_range = value.float(),
            Field::NavigationDistance => s.navigation_distance = value.float(),
            Field::NavigationTime => s.navigation_time = value.float(),
            Field::NavigationSpeedLimit => s.navigation_speed_limit = value.float(),
            Field::GameTime => s.game_time = value.u32(),
            Field::LocalScale => s.local_scale = value.float(),
            Field::DashboardBacklight => s.dashboard_backlight = value.float(),
            Field::ElectricEnabled => s.electric_enabled = value.bool(),
            Field::EngineEnabled => s.engine_enabled = value.bool(),
            Field::ParkingBrake => s.parking_brake = value.bool(),
            Field::LowBeam => s.low_beam = value.bool(),
            Field::HighBeam => s.high_beam = value.bool(),
            Field::LeftBlinker => s.left_blinker = value.bool(),
            Field::RightBlinker => s.right_blinker = value.bool(),
            Field::HeadOffset => {
                let p = value.fplacement();
                (s.head_x, s.head_y, s.head_z) = (p.x, p.y, p.z);
                (s.head_heading, s.head_pitch) = (p.heading, p.pitch);
            }
            Field::Steering => s.steering = value.float(),
            Field::Brake => s.brake = value.float(),
            Field::BrakeTemperature => s.brake_temperature = value.float(),
            Field::InputBrake => s.input_brake = value.float(),
            Field::MultiplayerTimeOffset => s.multiplayer_time_offset = value.s32(),
            Field::Wear(part) => s.wear[part] = value.float(),
            Field::FuelWarning => s.fuel_warning = value.bool(),
            Field::RestStop => (s.rest_stop, s.rest_known) = (value.s32(), true),
            Field::TrailerConnected | Field::TrailerPlacement => {}
        }
    }
}

fn reset(s: &mut Telemetry, field: Field) {
    let d = Telemetry::default();
    match field {
        Field::Placement => {
            (s.x, s.y, s.z) = (d.x, d.y, d.z);
            (s.heading, s.pitch, s.roll) = (d.heading, d.pitch, d.roll);
        }
        Field::Speed => s.speed = d.speed,
        Field::EngineRpm => s.engine_rpm = d.engine_rpm,
        Field::Gear => s.gear = d.gear,
        Field::DisplayedGear => s.displayed_gear = d.displayed_gear,
        Field::CruiseControl => s.cruise_control_speed = d.cruise_control_speed,
        Field::Fuel => s.fuel = d.fuel,
        Field::FuelRange => s.fuel_range = d.fuel_range,
        Field::NavigationDistance => s.navigation_distance = d.navigation_distance,
        Field::NavigationTime => s.navigation_time = d.navigation_time,
        Field::NavigationSpeedLimit => s.navigation_speed_limit = d.navigation_speed_limit,
        Field::GameTime => s.game_time = d.game_time,
        Field::LocalScale => s.local_scale = d.local_scale,
        Field::DashboardBacklight => s.dashboard_backlight = d.dashboard_backlight,
        Field::ElectricEnabled => s.electric_enabled = d.electric_enabled,
        Field::EngineEnabled => s.engine_enabled = d.engine_enabled,
        Field::ParkingBrake => s.parking_brake = d.parking_brake,
        Field::LowBeam => s.low_beam = d.low_beam,
        Field::HighBeam => s.high_beam = d.high_beam,
        Field::LeftBlinker => s.left_blinker = d.left_blinker,
        Field::RightBlinker => s.right_blinker = d.right_blinker,
        Field::HeadOffset => {
            (s.head_x, s.head_y, s.head_z) = (d.head_x, d.head_y, d.head_z);
            (s.head_heading, s.head_pitch) = (d.head_heading, d.head_pitch);
        }
        Field::Steering => s.steering = d.steering,
        Field::Brake => s.brake = d.brake,
        Field::BrakeTemperature => s.brake_temperature = d.brake_temperature,
        Field::InputBrake => s.input_brake = d.input_brake,
        Field::MultiplayerTimeOffset => s.multiplayer_time_offset = d.multiplayer_time_offset,
        Field::Wear(part) => s.wear[part] = d.wear[part],
        Field::FuelWarning => s.fuel_warning = d.fuel_warning,
        Field::RestStop => (s.rest_stop, s.rest_known) = (d.rest_stop, d.rest_known),
        Field::TrailerConnected | Field::TrailerPlacement => {}
    }
}
