//! What the reversing camera needs of the truck and its trailer (the cab page's `reversing`): how far the steered
//! wheels are turned, where the axles, the fifth wheel and the rear end are, and where the trailer is. The SDK
//! says the wheels (`wheel.position`, `wheel.steerable`, indexed, in the vehicle's own space: x right, y up, z
//! back) and the hooks (`hook.position`) in each vehicle's configuration, but no vehicle's length: its rear end
//! is worked out from its last axle ([`rear_overhang`]).

use std::sync::Mutex;

use crate::scs::ScsNamedValue;

/// The most wheels read of a vehicle (the SDK's own limit is 14 on a truck; trailers have fewer).
pub const MOST_WHEELS: usize = 16;

/// A vehicle's layout, as its configuration says it (metres, its own space: z back).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    /// The steered axle (the middle of the steered wheels ahead of the vehicle's middle; 0: none steer).
    pub front_axle_z: f32,
    /// The axle it turns about: the middle of the wheels that don't steer.
    pub rear_axle_z: f32,
    /// The hook: a truck's fifth wheel, a trailer's kingpin.
    pub hook_z: f32,
    /// Its rear end.
    pub rear_z: f32,
    /// Half its width, at its widest wheels' outer sides.
    pub half_width: f32,
    /// Where the ground is under it, up its own y: its wheels' middles less their radius (0: not said). Its origin
    /// isn't on the ground on every truck (owner 2026-11-14: our camera, 0.85 m up from it, was at the ground on a
    /// Scania).
    pub ground_y: f32,
}

/// Each wheel: where it is, and whether it steers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wheel {
    pub at: [f32; 3],
    pub steers: bool,
    /// Its radius (metres; 0: not said).
    pub radius: f32,
}

/// Half a tyre's width, out of a wheel's middle (metres).
const HALF_TYRE: f32 = 0.16;

/// How far a vehicle goes on behind its last axle (metres): a truck's chassis ends a metre or so behind it; a
/// trailer's rear overhang grows with how far its axles are from its kingpin (a 13.6 m European trailer ends
/// some 3.4 m behind its last axle, 9 m from its kingpin; a container chassis a metre and a bit behind, 5 m
/// from it).
pub fn rear_overhang(trailer: bool, last_axle_z: f32, hook_z: f32) -> f32 {
    if !trailer {
        return 1.1;
    }
    (0.33 * (last_axle_z - hook_z)).clamp(1.0, 3.5)
}

/// A vehicle's layout from its wheels and its hook; `None` without wheels.
pub fn layout(wheels: &[Wheel], hook: [f32; 3], trailer: bool) -> Option<Layout> {
    if wheels.is_empty() {
        return None;
    }
    let zs = wheels.iter().map(|wheel| wheel.at[2]);
    let first = zs.clone().fold(f32::INFINITY, f32::min);
    let last = zs.fold(f32::NEG_INFINITY, f32::max);
    let middle = (first + last) / 2.0;
    let mean = |pick: &dyn Fn(&Wheel) -> bool| {
        let picked: Vec<f32> = wheels.iter().filter(|wheel| pick(wheel)).map(|wheel| wheel.at[2]).collect();
        (!picked.is_empty()).then(|| picked.iter().sum::<f32>() / picked.len() as f32)
    };
    let front_axle_z = mean(&|wheel| wheel.steers && wheel.at[2] < middle).unwrap_or(0.0);
    // The axle it turns about: the wheels that don't steer (a trailer's, all of them if every one is said to steer).
    let rear_axle_z = mean(&|wheel| !wheel.steers).or_else(|| mean(&|_| true)).unwrap_or(0.0);
    let half_width = wheels.iter().map(|wheel| wheel.at[0].abs()).fold(0.0, f32::max) + HALF_TYRE;
    Some(Layout {
        front_axle_z,
        rear_axle_z,
        hook_z: hook[2],
        rear_z: last + rear_overhang(trailer, last, hook[2]),
        half_width: half_width.max(1.0),
        ground_y: {
            let said: Vec<f32> =
                wheels.iter().filter(|wheel| wheel.radius > 0.05).map(|wheel| wheel.at[1] - wheel.radius).collect();
            if said.is_empty() { 0.0 } else { said.iter().sum::<f32>() / said.len() as f32 }
        },
    })
}

/// A vehicle's configuration as it comes: its wheels and hook, read from the attributes.
#[derive(Clone, Copy, Default)]
pub struct Configured {
    pub wheels: [Wheel; MOST_WHEELS],
    pub count: usize,
    pub hook: [f32; 3],
}

impl Configured {
    /// Takes in one of the configuration's attributes, if it's one of a wheel's or the hook.
    ///
    /// # Safety
    /// `attribute` is the SDK's, its name nul-terminated, while the configuration's callback runs.
    pub unsafe fn take(&mut self, attribute: &ScsNamedValue) {
        // SAFETY: as this function's contract says.
        let name = unsafe { std::ffi::CStr::from_ptr(attribute.name) }.to_bytes();
        let index = attribute.index as usize;
        match name {
            b"wheels.count" => self.count = (attribute.value.u32() as usize).min(MOST_WHEELS),
            b"hook.position" => self.hook = attribute.value.fvector().unwrap_or_default(),
            b"wheel.position" if index < MOST_WHEELS => {
                self.wheels[index].at = attribute.value.fvector().unwrap_or_default()
            }
            b"wheel.steerable" if index < MOST_WHEELS => self.wheels[index].steers = attribute.value.bool(),
            b"wheel.radius" if index < MOST_WHEELS => self.wheels[index].radius = attribute.value.float(),
            _ => {}
        }
    }

    pub fn layout(&self, trailer: bool) -> Option<Layout> {
        layout(&self.wheels[..self.count], self.hook, trailer)
    }
}

/// The truck's layout and which of its wheels steer, and the trailer's layout, as their configurations last said.
pub struct Vehicles {
    pub truck: Layout,
    pub steers: [bool; MOST_WHEELS],
    pub trailer: Layout,
    /// How far each of the truck's wheels is turned (turns, the SDK's `truck.wheel.steering`).
    pub steering: [f32; MOST_WHEELS],
    /// The suspension (the phone's Suspensão app): which of the truck's wheels are its front axle's, how many
    /// wheels each has, and how far each wheel's spring is pressed in (metres, the SDK's `wheel.suspension.deflection`).
    pub front: [bool; MOST_WHEELS],
    pub truck_wheels: usize,
    pub trailer_wheels: usize,
    /// Exact wheel hub positions (`wheel.position`, local x right, y up, z back) for truck and trailer (brake smoke).
    pub truck_wheel_at: [[f32; 3]; MOST_WHEELS],
    pub trailer_wheel_at: [[f32; 3]; MOST_WHEELS],
    pub deflection: [f32; MOST_WHEELS],
    pub trailer_deflection: [f32; MOST_WHEELS],
}

pub static VEHICLES: Mutex<Vehicles> = Mutex::new(Vehicles {
    truck: Layout { front_axle_z: 0.0, rear_axle_z: 0.0, hook_z: 0.0, rear_z: 0.0, half_width: 0.0, ground_y: 0.0 },
    steers: [false; MOST_WHEELS],
    trailer: Layout { front_axle_z: 0.0, rear_axle_z: 0.0, hook_z: 0.0, rear_z: 0.0, half_width: 0.0, ground_y: 0.0 },
    steering: [0.0; MOST_WHEELS],
    front: [false; MOST_WHEELS],
    truck_wheels: 0,
    trailer_wheels: 0,
    truck_wheel_at: [[0.0; 3]; MOST_WHEELS],
    trailer_wheel_at: [[0.0; 3]; MOST_WHEELS],
    deflection: [0.0; MOST_WHEELS],
    trailer_deflection: [0.0; MOST_WHEELS],
});

impl Vehicles {
    /// The suspension's three groups (`Telemetry::suspension`): the front axle's wheels, the truck's others, the
    /// trailer's; each the mean of its wheels' (-1 with none).
    pub fn suspension(&self, trailer: bool) -> [f32; 3] {
        let mean = |values: &mut dyn Iterator<Item = f32>| {
            let (sum, count) = values.fold((0.0, 0), |(sum, count), v| (sum + v, count + 1));
            if count == 0 { -1.0 } else { sum / count as f32 }
        };
        let count = self.truck_wheels.min(MOST_WHEELS);
        let front = mean(&mut (0..count).filter(|&at| self.front[at]).map(|at| self.deflection[at]));
        let rear = mean(&mut (0..count).filter(|&at| !self.front[at]).map(|at| self.deflection[at]));
        let trailer_count = if trailer { self.trailer_wheels.min(MOST_WHEELS) } else { 0 };
        let trailer = mean(&mut (0..trailer_count).map(|at| self.trailer_deflection[at]));
        [front, rear, trailer]
    }

    /// How far the steered wheels ahead are turned (radians, left positive): the mean of theirs.
    pub fn wheel_steering(&self) -> f32 {
        let turned: Vec<f32> = (0..MOST_WHEELS).filter(|&at| self.steers[at]).map(|at| self.steering[at]).collect();
        if turned.is_empty() {
            return 0.0;
        }
        turned.iter().sum::<f32>() / turned.len() as f32 * std::f32::consts::TAU
    }
}

/// The truck's configuration, read: its layout, and which wheels steer (those ahead of its middle).
pub fn truck_configured(configured: &Configured) {
    let mut vehicles = VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
    vehicles.truck = configured.layout(false).unwrap_or_default();
    let front = vehicles.truck.front_axle_z;
    let rear = vehicles.truck.rear_axle_z;
    for at in 0..MOST_WHEELS {
        let wheel = configured.wheels[at];
        // Steered and nearer the steered axle than the one it turns about: a tag axle steering behind isn't one.
        vehicles.steers[at] =
            at < configured.count && wheel.steers && (wheel.at[2] - front).abs() < (wheel.at[2] - rear).abs();
    }
    // The front axle's wheels (the suspension's front): nearer the steered axle than the one it turns about.
    vehicles.truck_wheels = configured.count;
    for at in 0..MOST_WHEELS {
        let wheel = configured.wheels[at];
        vehicles.front[at] = at < configured.count && (wheel.at[2] - front).abs() < (wheel.at[2] - rear).abs();
        vehicles.truck_wheel_at[at] = if at < configured.count { wheel.at } else { [0.0; 3] };
    }
    crate::log!("reversing: truck laid out as {:?}", vehicles.truck);
}

/// The first trailer's configuration, read (none: no attributes).
pub fn trailer_configured(configured: &Configured) {
    let mut vehicles = VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
    vehicles.trailer = configured.layout(true).unwrap_or_default();
    vehicles.trailer_wheels = configured.count;
    for at in 0..MOST_WHEELS {
        vehicles.trailer_wheel_at[at] = if at < configured.count { configured.wheels[at].at } else { [0.0; 3] };
    }
    crate::log!("reversing: trailer laid out as {:?}", vehicles.trailer);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(x: f32, z: f32, steers: bool) -> Wheel {
        Wheel { at: [x, 0.5, z], steers, radius: 0.5 }
    }

    #[test]
    fn lays_a_tractor_out_from_its_wheels() {
        // A 6x4: the steered axle ahead, a tandem behind.
        let wheels = [
            wheel(-1.0, -1.8, true),
            wheel(1.0, -1.8, true),
            wheel(-0.9, 1.8, false),
            wheel(0.9, 1.8, false),
            wheel(-0.9, 3.1, false),
            wheel(0.9, 3.1, false),
        ];
        let truck = layout(&wheels, [0.0, 1.2, 2.2], false).unwrap();
        assert_eq!(truck.front_axle_z, -1.8);
        assert!((truck.rear_axle_z - 2.45).abs() < 1e-4);
        assert!((truck.rear_z - 4.2).abs() < 1e-4);
        assert!((truck.half_width - 1.16).abs() < 1e-4);
    }

    #[test]
    fn a_trailers_rear_overhang_grows_with_its_length() {
        // A European trailer: three axles 7.7–10.3 m behind its kingpin, its rear some 3.4 m behind.
        let wheels = [wheel(-0.9, 7.7, false), wheel(-0.9, 9.0, false), wheel(0.9, 10.3, false)];
        let trailer = layout(&wheels, [0.0, 1.1, 0.0], true).unwrap();
        assert!((trailer.rear_axle_z - 9.0).abs() < 1e-4);
        assert!((trailer.rear_z - 10.3 - 3.399).abs() < 0.01, "{}", trailer.rear_z);
        // Short: a metre at least.
        assert_eq!(rear_overhang(true, 2.0, 0.0), 1.0);
    }

    #[test]
    fn the_ground_is_under_the_wheels() {
        // Wheels of 0.5 m radius with their middles 0.5 m up: the ground at the origin; 0.2 m up, it is 0.3 below.
        let level = layout(&[wheel(1.0, 2.0, false)], [0.0; 3], false).unwrap();
        assert!(level.ground_y.abs() < 1e-5);
        let low = layout(&[Wheel { at: [1.0, 0.2, 2.0], steers: false, radius: 0.5 }], [0.0; 3], false).unwrap();
        assert!((low.ground_y + 0.3).abs() < 1e-5);
    }
}
