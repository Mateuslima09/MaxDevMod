import type { Telemetry } from '../../shared/host'
import { backOf, type Rear, rightOf, type Vec } from './geometry'

/**
 * The parking sensors across the rear: four in the bumper, as on a real truck, each looking back in a cone and
 * telling how near the nearest thing in it is. What's there is what WestPlay knows of the world round the truck
 * (the map's buildings, TruckersMP's players: `Obstacle`), found by rays across each sensor's cone.
 *
 * And the beeping they make: silent far off, then slow beeps that come quicker, and louder, as the rear nears
 * something, until the tone holds steady right up against it.
 */

/** Something the rear can back into: an outline (a building's walls, a ring of world points), or a round thing. */
export type Obstacle = { kind: 'outline'; ring: Vec[] } | { kind: 'round'; at: Vec; radius: number }

export interface Sensor {
  /** Across the rear, from its middle: -1 its left corner, 1 its right (as seen from the cab). */
  across: number
  /** How far it sees, metres. */
  range: number
  /** Half its cone, radians, and which way its middle looks, out from straight back (radians, right positive). */
  halfCone: number
  aim: number
}

/** The four: the corners see less far and look out to the sides a little, as real ones are set. */
export const SENSORS: readonly Sensor[] = [
  { across: -0.92, range: 2.0, halfCone: 0.6, aim: -0.35 },
  { across: -0.35, range: 3.0, halfCone: 0.5, aim: 0 },
  { across: 0.35, range: 3.0, halfCone: 0.5, aim: 0 },
  { across: 0.92, range: 2.0, halfCone: 0.6, aim: 0.35 },
]

/** Rays across each sensor's cone. */
const RAYS = 9

/** Where a ray from `from` along `dir` (a unit vector) meets the segment a–b, metres along it (`null`: it doesn't). */
function rayToSegment(from: Vec, dir: Vec, a: Vec, b: Vec): number | null {
  const ex = b[0] - a[0]
  const ez = b[1] - a[1]
  const denominator = dir[0] * ez - dir[1] * ex
  if (Math.abs(denominator) < 1e-9) return null
  const ax = a[0] - from[0]
  const az = a[1] - from[1]
  const along = (ax * ez - az * ex) / denominator
  const on = (ax * dir[1] - az * dir[0]) / denominator
  return along >= 0 && on >= 0 && on <= 1 ? along : null
}

/** Where a ray meets a circle, metres along it (`null`: it doesn't; 0: it starts in it). */
function rayToCircle(from: Vec, dir: Vec, at: Vec, radius: number): number | null {
  const ox = from[0] - at[0]
  const oz = from[1] - at[1]
  const b = ox * dir[0] + oz * dir[1]
  const c = ox * ox + oz * oz - radius * radius
  if (c <= 0) return 0
  const discriminant = b * b - c
  if (discriminant < 0) return null
  const along = -b - Math.sqrt(discriminant)
  return along >= 0 ? along : null
}

/** The nearest thing along a ray, metres (`Infinity`: nothing). */
function castRay(from: Vec, dir: Vec, obstacles: readonly Obstacle[]): number {
  let nearest = Infinity
  for (const obstacle of obstacles) {
    if (obstacle.kind === 'round') {
      const along = rayToCircle(from, dir, obstacle.at, obstacle.radius)
      if (along !== null) nearest = Math.min(nearest, along)
      continue
    }
    const ring = obstacle.ring
    for (let at = 0; at < ring.length; at++) {
      const along = rayToSegment(from, dir, ring[at], ring[(at + 1) % ring.length])
      if (along !== null) nearest = Math.min(nearest, along)
    }
  }
  return nearest
}

/** Where a sensor sits in the world, on the rear's bumper. */
export function sensorAt(rear: Rear, sensor: Sensor): Vec {
  const right = rightOf(rear.heading)
  return [
    rear.at[0] + right[0] * rear.halfWidth * sensor.across,
    rear.at[1] + right[1] * rear.halfWidth * sensor.across,
  ]
}

/** The obstacles that might be near the rear: those with a point, or (an outline) a side, within `reach` of it. */
export function obstaclesNear(rear: Rear, obstacles: readonly Obstacle[], reach: number): Obstacle[] {
  const [x, z] = rear.at
  return obstacles.filter((obstacle) => {
    if (obstacle.kind === 'round') return Math.hypot(obstacle.at[0] - x, obstacle.at[1] - z) < reach + obstacle.radius
    let [minX, minZ, maxX, maxZ] = [Infinity, Infinity, -Infinity, -Infinity]
    for (const [px, pz] of obstacle.ring) {
      minX = Math.min(minX, px)
      maxX = Math.max(maxX, px)
      minZ = Math.min(minZ, pz)
      maxZ = Math.max(maxZ, pz)
    }
    return x > minX - reach && x < maxX + reach && z > minZ - reach && z < maxZ + reach
  })
}

/**
 * How far each sensor sees the nearest thing in its cone (metres, `null`: nothing in its range), in [`SENSORS`]'
 * order.
 */
export function readSensors(rear: Rear, obstacles: readonly Obstacle[]): (number | null)[] {
  const near = obstaclesNear(rear, obstacles, Math.max(...SENSORS.map((sensor) => sensor.range)) + 1)
  // Straight back from the rear, and its right: a ray turned `angle` to the right of straight back.
  const back = backOf(rear.heading)
  const right = rightOf(rear.heading)
  return SENSORS.map((sensor) => {
    if (near.length === 0) return null
    const from = sensorAt(rear, sensor)
    let nearest = Infinity
    for (let ray = 0; ray < RAYS; ray++) {
      const angle = sensor.aim + sensor.halfCone * ((2 * ray) / (RAYS - 1) - 1)
      const dir: Vec = [
        back[0] * Math.cos(angle) + right[0] * Math.sin(angle),
        back[1] * Math.cos(angle) + right[1] * Math.sin(angle),
      ]
      nearest = Math.min(nearest, castRay(from, dir, near))
    }
    return nearest <= sensor.range ? nearest : null
  })
}

/** `Telemetry.rays` when the game's physics answers the plugin's rays (crates/plugin ray.rs `WORKING`). */
export const RAYS_WORKING = 1

/**
 * The readings with the game's own (its physics: whatever the truck would bump into, the map's scenery, the traffic
 * and trailers included, on any map), where the plugin's rays work: each sensor the nearer of the two.
 */
export function withRays(
  readings: readonly (number | null)[],
  telemetry: Pick<Telemetry, 'rays' | 'raySensors'>,
): (number | null)[] {
  if (telemetry.rays !== RAYS_WORKING || !Array.isArray(telemetry.raySensors)) return [...readings]
  return readings.map((reading, index) => {
    const ray = telemetry.raySensors[index]
    if (ray === undefined || !Number.isFinite(ray) || ray < 0) return reading
    return reading === null ? ray : Math.min(reading, ray)
  })
}

/** The nearest of the sensors' readings (`null`: none sees anything). */
export function nearestOf(readings: readonly (number | null)[]): number | null {
  const seen = readings.filter((reading): reading is number => reading !== null)
  return seen.length ? Math.min(...seen) : null
}

/** How the sensors sound for the nearest reading: a beep every `every` ms (`0`: one steady tone), this loud
 * (0–1); `null`: silent. */
export interface Beeping {
  every: number
  loudness: number
}

/** The beeping starts this far off (m), and holds a steady tone this near. */
export const BEEP_FROM = 3
export const STEADY_AT = 0.35
/** The time between beeps at the far end and near the steady tone, ms. */
const SLOWEST = 950
const QUICKEST = 110
/** How loud at the far end and at the steady tone. */
const QUIETEST = 0.35
const LOUDEST = 1

export function beepingFor(distance: number | null): Beeping | null {
  if (distance === null || distance > BEEP_FROM) return null
  if (distance <= STEADY_AT) return { every: 0, loudness: LOUDEST }
  // Near: 0, far: 1. The beeps quicken faster as it nears (squared), as a real system's do.
  const far = (distance - STEADY_AT) / (BEEP_FROM - STEADY_AT)
  return {
    every: Math.round(QUICKEST + (SLOWEST - QUICKEST) * far * far),
    loudness: LOUDEST - (LOUDEST - QUIETEST) * far,
  }
}

/** How a reading is shown: the colour of its band (red near, then yellow, then green). */
export type Zone = 'red' | 'yellow' | 'green'

export function zoneOf(distance: number): Zone {
  if (distance < 0.8) return 'red'
  if (distance < 1.6) return 'yellow'
  return 'green'
}
