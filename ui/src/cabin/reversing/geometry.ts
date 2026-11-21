import type { Telemetry } from '../../shared/host'

/**
 * The reversing camera's geometry, in the game's ground plane (metres, x east, z south; headings in radians,
 * counterclockwise seen from above, 0 facing north): where the vehicle's rear is, where the camera sits on it,
 * and where the rear goes as the truck backs up with its wheels turned as they are now.
 *
 * A vehicle's own space is the SDK's: x right, z back (crates/plugin reversing.rs lays the truck and its
 * trailer out in it). The truck turns about its rear axle, its steered axle ahead of it (a bicycle: the
 * turning's curvature is tan(steer) / wheelbase). A trailer is drawn by its kingpin on the truck's fifth wheel
 * and turns about its own axle, which can't slide sideways: backing up, it swings out the other way from the
 * truck's turn, further and further (a jackknife), as a real one does. Those are followed step by step.
 */

export type Vec = [x: number, z: number]

/** A vehicle's heading as the SDK says it (turns) in radians. */
export const radians = (turns: number) => turns * 2 * Math.PI

/** Which way a vehicle's own back and right point in the world, turned `heading`. */
export const backOf = (heading: number): Vec => [Math.sin(heading), Math.cos(heading)]
export const rightOf = (heading: number): Vec => [Math.cos(heading), -Math.sin(heading)]

const add = (a: Vec, b: Vec, scale = 1): Vec => [a[0] + b[0] * scale, a[1] + b[1] * scale]
const dot = (a: Vec, b: Vec) => a[0] * b[0] + a[1] * b[1]

/** What's known of the truck and its trailer to reverse them. */
export interface Rig {
  /** The truck's rear axle (world) and heading. */
  axle: Vec
  heading: number
  /** How far the steered wheels are turned (radians, left positive). */
  steer: number
  /** From the steered axle back to the rear axle, metres. */
  wheelbase: number
  /** From the rear axle back to the fifth wheel (negative: ahead of it), and to the truck's rear end. */
  hookBack: number
  rearBack: number
  halfWidth: number
  /** The trailer, coupled: its heading, from its kingpin back to its axle and to its rear end, its half width. */
  trailer: { heading: number; axleBack: number; rearBack: number; halfWidth: number } | null
}

/** Below this wheelbase or trailer length (m), the configuration isn't believed: a typical one instead. */
const LEAST_LENGTH = 1.5
const TYPICAL_WHEELBASE = 3.8
const TYPICAL_TRAILER_AXLE = 9
/** A truck laid out with no width (its configuration not read yet): a truck's. */
const TYPICAL_HALF_WIDTH = 1.27

/** The rig from the telemetry (`null`: no truck yet, or a plugin that doesn't lay it out). */
export function rigOf(t: Telemetry): Rig | null {
  if (t.halfWidth === undefined) return null
  const heading = radians(t.heading)
  const axle = add([t.x, t.z], backOf(heading), t.rearAxleZ)
  const wheelbase = t.rearAxleZ - t.frontAxleZ
  const trailerAxle = t.trailerAxleZ - t.trailerHookZ
  return {
    axle,
    heading,
    steer: t.wheelSteering,
    wheelbase: wheelbase > LEAST_LENGTH ? wheelbase : TYPICAL_WHEELBASE,
    hookBack: t.hookZ - t.rearAxleZ,
    rearBack: Math.max(0.5, t.rearZ - t.rearAxleZ),
    halfWidth: t.halfWidth > 0 ? t.halfWidth : TYPICAL_HALF_WIDTH,
    trailer: t.trailerConnected
      ? {
          heading: radians(t.trailerHeading),
          axleBack: trailerAxle > LEAST_LENGTH ? trailerAxle : TYPICAL_TRAILER_AXLE,
          rearBack: Math.max(trailerAxle, t.trailerRearZ - t.trailerHookZ),
          halfWidth: t.trailerHalfWidth > 0 ? t.trailerHalfWidth : TYPICAL_HALF_WIDTH,
        }
      : null,
  }
}

/** Where the rig's rear end is: its middle, which way it faces back, and its half width. */
export interface Rear {
  at: Vec
  heading: number
  halfWidth: number
}

/** The rear of the rig: the trailer's, coupled, else the truck's. */
export function rearOf(rig: Rig): Rear {
  if (!rig.trailer) {
    return { at: add(rig.axle, backOf(rig.heading), rig.rearBack), heading: rig.heading, halfWidth: rig.halfWidth }
  }
  const kingpin = add(rig.axle, backOf(rig.heading), rig.hookBack)
  return {
    at: add(kingpin, backOf(rig.trailer.heading), rig.trailer.rearBack),
    heading: rig.trailer.heading,
    halfWidth: rig.trailer.halfWidth,
  }
}

/** The rear's two corners (left, right as seen from the cab) and its middle, and how far it has gone back. */
export interface PathPoint {
  left: Vec
  middle: Vec
  right: Vec
  /** Along the ground the rear's middle has gone, metres. */
  travelled: number
  /** The trailer's angle to the truck (radians), at this point: past `JACKKNIFE` the path ends. */
  articulation: number
}

/** A step of the truck's backing up, metres. */
const STEP = 0.1
/** Past this angle between truck and trailer (radians, 65°), the path goes no further: the rig would fold. */
export const JACKKNIFE = (65 * Math.PI) / 180

/** Where the truck (its rear axle, its heading) and its trailer (its heading) are as the rig backs up. */
export interface Pose {
  axle: Vec
  heading: number
  trailerHeading: number
}

/**
 * The rig backed up `distance` metres of the truck's axle, its wheels turned as `rig` has them: the truck as a
 * bicycle (it turns tan(steer) / wheelbase per metre), the trailer pulled by its kingpin without sliding sideways.
 * One step of [`reversePath`] (and of the browser preview's backing up, preview.ts).
 */
export function backUp(rig: Rig, pose: Pose, distance: number): Pose {
  const curvature = Math.tan(Math.max(-1.2, Math.min(1.2, rig.steer))) / rig.wheelbase
  // Backing: the axle goes back along the truck's heading; the truck turns the other way from going ahead.
  const v = -distance
  const turn = v * curvature
  const forward: Vec = [-Math.sin(pose.heading), -Math.cos(pose.heading)]
  let trailerHeading = pose.trailerHeading
  if (rig.trailer) {
    // The fifth wheel's motion: the axle's, and the truck turning about it.
    const hookMotion = add([forward[0] * v, forward[1] * v], rightOf(pose.heading), rig.hookBack * turn)
    trailerHeading -= dot(hookMotion, rightOf(trailerHeading)) / rig.trailer.axleBack
  }
  return { axle: add(pose.axle, forward, v), heading: pose.heading + turn, trailerHeading }
}

/** Wraps an angle into -π..π. */
export const wrap = (angle: number) => Math.atan2(Math.sin(angle), Math.cos(angle))

/**
 * Where the rear goes as the truck backs up `length` metres (the rear's own travel), its wheels as they are now:
 * a point every [`STEP`] of the truck's, the first where the rear is now.
 */
export function reversePath(rig: Rig, length: number): PathPoint[] {
  let pose: Pose = { axle: rig.axle, heading: rig.heading, trailerHeading: rig.trailer?.heading ?? 0 }
  const point = (travelled: number): PathPoint => {
    const { axle, heading, trailerHeading } = pose
    const rear = rearOf({ ...rig, axle, heading, trailer: rig.trailer && { ...rig.trailer, heading: trailerHeading } })
    const right = rightOf(rear.heading)
    return {
      left: add(rear.at, right, -rear.halfWidth),
      middle: rear.at,
      right: add(rear.at, right, rear.halfWidth),
      travelled,
      articulation: rig.trailer ? wrap(trailerHeading - heading) : 0,
    }
  }
  const points = [point(0)]
  // The truck's own steps go on as long as the rear's travel is short of `length` (a trailer's rear can go
  // further or less far than the truck), with a bound for a rear that barely moves (turned about it).
  for (let steps = 0; points.at(-1)!.travelled < length && steps < (length / STEP) * 4; steps++) {
    pose = backUp(rig, pose, STEP)
    const last = points.at(-1)!
    const next = point(0)
    next.travelled = last.travelled + Math.hypot(next.middle[0] - last.middle[0], next.middle[1] - last.middle[1])
    points.push(next)
    if (Math.abs(next.articulation) > JACKKNIFE) break
  }
  return points
}

/** Where the camera is and looks: over the rear's middle, facing back, tilted down. */
export interface CameraPlace {
  at: Vec
  /** Its height over the ground, metres. */
  height: number
  /** Which way it faces (radians, as headings): the rear's back. */
  facing: number
}

/** The camera's height: over a trailer's doors, or on a truck's chassis looking down to the fifth wheel's end. */
export const TRAILER_CAMERA_HEIGHT = 3.6
export const TRUCK_CAMERA_HEIGHT = 1.6

export function cameraOf(rig: Rig): CameraPlace {
  const rear = rearOf(rig)
  return {
    at: rear.at,
    height: rig.trailer ? TRAILER_CAMERA_HEIGHT : TRUCK_CAMERA_HEIGHT,
    facing: wrap(rear.heading + Math.PI),
  }
}

/** A heading (radians, counterclockwise from north) as MapLibre's bearing (degrees, clockwise from north). */
export const bearingOf = (heading: number) => ((((-heading * 180) / Math.PI) % 360) + 360) % 360
