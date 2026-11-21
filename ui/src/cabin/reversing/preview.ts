import { isInHost, type Telemetry } from '../../shared/host'
import { useLive } from '../state/store'
import { backOf, backUp, type Pose, rearOf, rigOf, type Vec } from './geometry'
import { nearestOf, type Obstacle, readSensors } from './sensor'
import { cameraView } from './view'

/**
 * The reversing camera in a plain browser (`?reverse`), with no game: a truck with its trailer backs up slowly
 * towards a wall of the page's own, its wheel turning from one side to the other, slowing as the sensors near
 * it; it stops against it, waits and starts again. For seeing the screen, hearing the beeps and taking
 * screenshots without the game. With no map of the player's, the camera shows its grid (view.ts `drawGrid`).
 *
 * - `?reverse=truck`: the truck alone, no trailer.
 * - `?reverse=night`: at night (the camera's night look). Both together: `?reverse=truck,night`.
 * - `?reverse=still`: stopped 1.2 m from the wall, the wheel turned left, to look at the lines and the panel.
 *
 * Nothing here is ever in the app.
 */
export function startReversePreview(): (() => void) | undefined {
  if (isInHost()) return undefined
  const query = new URLSearchParams(location.search)
  if (!query.has('reverse')) return undefined
  const options = new Set((query.get('reverse') ?? '').split(','))
  const preview = new ReversePreview({
    trailer: !options.has('truck'),
    night: options.has('night'),
    still: options.has('still'),
  })
  cameraView().extraObstacles = preview.obstacles
  useLive.setState({ telemetry: preview.telemetry(), gameShown: true })
  let last = performance.now()
  const timer = setInterval(() => {
    const now = performance.now()
    preview.advance(Math.min(0.2, (now - last) / 1000))
    last = now
    useLive.setState({ telemetry: preview.telemetry() })
  }, TICK_MS)
  return () => {
    clearInterval(timer)
    cameraView().extraObstacles = []
  }
}

/** As often as the host sends the telemetry in reverse (crates/host `TELEMETRY_REVERSING_MS`). */
const TICK_MS = 33

/** The truck: a 6x4 tractor laid out as the plugin says it (metres, z back from its origin at the steered axle). */
const TRUCK = { frontAxleZ: 0, rearAxleZ: 3.9, hookZ: 3.6, rearZ: 4.9, halfWidth: 1.27 }
/** A 13.6 m semi-trailer: its kingpin, its axles' middle and its rear end in its own space. */
const TRAILER = { hookZ: -5.6, axleZ: 3.6, rearZ: 7.2, halfWidth: 1.3 }

/** How far behind the rear the wall stands at the start (m), and how wide and thick it is. */
const WALL_GAP = 14
const WALL_HALF_WIDTH = 9
const WALL_THICKNESS = 0.6
/** A post near the right-hand corner, a little short of the wall (the corner sensors see it first). */
const POST_RIGHT = 3.4
const POST_SHORT = 2.2
const POST_RADIUS = 0.35

/** Backing speed far off (m/s), the least as it nears, and how far it slows from (m). */
const FAR_SPEED = 1.4
const NEAR_SPEED = 0.25
const SLOW_FROM = 3
/** It stops this near (m), waits this long (s), and starts again from the beginning. */
const STOP_AT = 0.3
const WAIT = 2.5
/** The wheel's swing: how far either way (radians) and how long one swing lasts (s). */
const STEER_MOST = 0.28
const STEER_PERIOD = 10

class ReversePreview {
  readonly obstacles: Obstacle[]
  private pose: Pose = { axle: [0, TRUCK.rearAxleZ], heading: 0, trailerHeading: 0 }
  private time = 0
  private speed = 0
  private waiting = 0
  private readonly gameTime: number
  private readonly options: { trailer: boolean; night: boolean; still: boolean }

  constructor(options: { trailer: boolean; night: boolean; still: boolean }) {
    this.options = options
    this.gameTime = options.night ? 22 * 60 : 12 * 60 + 30
    // The rear at the start, straight back (+z with the truck facing north); the wall behind it.
    const rear = this.rear()
    const wallZ = rear[1] + (options.still ? 1.2 : WALL_GAP)
    this.obstacles = [
      {
        kind: 'outline',
        ring: [
          [-WALL_HALF_WIDTH, wallZ],
          [WALL_HALF_WIDTH, wallZ],
          [WALL_HALF_WIDTH, wallZ + WALL_THICKNESS],
          [-WALL_HALF_WIDTH, wallZ + WALL_THICKNESS],
          [-WALL_HALF_WIDTH, wallZ],
        ],
      },
      { kind: 'round', at: [POST_RIGHT, wallZ - POST_SHORT], radius: POST_RADIUS },
    ]
  }

  /** The wheel now: a slow swing from one side to the other (still: turned left). */
  private steer(): number {
    if (this.options.still) return STEER_MOST
    return STEER_MOST * Math.sin((this.time * 2 * Math.PI) / STEER_PERIOD)
  }

  private rear(): Vec {
    const rig = rigOf(this.telemetry())
    return rig ? rearOf(rig).at : [0, 0]
  }

  /** `seconds` on: backed up, slowing as the sensors near the wall; against it, waits and starts over. */
  advance(seconds: number) {
    this.time += seconds
    if (this.options.still) return
    if (this.waiting > 0) {
      this.waiting -= seconds
      if (this.waiting <= 0) {
        this.pose = { axle: [0, TRUCK.rearAxleZ], heading: 0, trailerHeading: 0 }
        this.time = 0
      }
      return
    }
    const rig = rigOf(this.telemetry())
    if (!rig) return
    const nearest = nearestOf(readSensors(rearOf(rig), this.obstacles))
    if (nearest !== null && nearest <= STOP_AT) {
      this.speed = 0
      this.waiting = WAIT
      return
    }
    const slowing = nearest === null ? 1 : Math.min(1, nearest / SLOW_FROM)
    this.speed = NEAR_SPEED + (FAR_SPEED - NEAR_SPEED) * slowing
    this.pose = backUp(rig, this.pose, this.speed * seconds)
  }

  /** The telemetry as the plugin would send it now. */
  telemetry(): Telemetry {
    const { axle, heading, trailerHeading } = this.pose
    // The truck's origin: its rear axle forward by its own distance back to it.
    const [x, z] = add(axle, backOf(heading), -TRUCK.rearAxleZ)
    const kingpin = add(axle, backOf(heading), TRUCK.hookZ - TRUCK.rearAxleZ)
    const [trailerX, trailerZ] = add(kingpin, backOf(trailerHeading), -TRAILER.hookZ)
    const turns = (angle: number) => angle / (2 * Math.PI)
    const trailer = this.options.trailer
    return {
      x,
      y: 0,
      z,
      heading: turns(heading),
      pitch: 0,
      roll: 0,
      speed: -this.speed,
      engineRpm: 750,
      gear: -1,
      displayedGear: -1,
      cruiseControlSpeed: 0,
      fuel: 600,
      fuelRange: 1500,
      navigationDistance: 0,
      navigationTime: 0,
      navigationSpeedLimit: 0,
      gameTime: this.gameTime,
      multiplayerTimeOffset: 0,
      localScale: 19,
      dashboardBacklight: this.options.night ? 1 : 0,
      electricEnabled: true,
      engineEnabled: true,
      parkingBrake: false,
      lowBeam: this.options.night,
      highBeam: false,
      leftBlinker: false,
      rightBlinker: false,
      paused: false,
      steering: this.steer() / 0.6,
      wheelSteering: this.steer(),
      ...TRUCK,
      trailerConnected: trailer,
      trailerX: trailer ? trailerX : 0,
      trailerY: 0,
      trailerZ: trailer ? trailerZ : 0,
      trailerHeading: trailer ? turns(trailerHeading) : 0,
      trailerHookZ: TRAILER.hookZ,
      trailerAxleZ: TRAILER.axleZ,
      trailerRearZ: TRAILER.rearZ,
      trailerHalfWidth: TRAILER.halfWidth,
      raySensors: [-1, -1, -1, -1],
      rays: 0,
      mirror: false,
    }
  }
}

const add = (a: Vec, b: Vec, scale: number): Vec => [a[0] + b[0] * scale, a[1] + b[1] * scale]
