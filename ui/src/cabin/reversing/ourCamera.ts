import type { Telemetry } from '../../shared/host'
import { backOf, type PathPoint, rearOf, reversePath, rightOf, rigOf, type Vec } from './geometry'
import { GUIDE_MARKS, pointAt, quad, towards } from './guides'

/**
 * Our own reversing camera, made by the plugin from one of the game's mirrors (crates/plugin rear_camera.rs,
 * owner 2026-11-14): where it is and how it sees, the same numbers as there, and the guide lines drawn over its
 * picture (the map's camera, view.ts, has its own). Keep these in step with rear_camera.rs.
 */
/** How wide it sees across the picture (degrees; rear_camera.rs WIDE), and how far down it looks (PITCH). */
export const WIDE = 120
export const PITCH = 22
/** Its height (m; HEIGHT_TRUCK, HEIGHT_TRAILER) and how far past the rear end it is (PAST_REAR). */
export const HEIGHT_TRUCK = 0.85
export const HEIGHT_TRAILER = 1.1
export const PAST_REAR = 0
/** How far ahead of a trailer's worked-out rear end it is (m; TRAILER_IN). */
export const TRAILER_IN = 0
/** Whether the picture shows the rig's right on the screen's right (a reversing camera's usual mirror picture). */
export const MIRRORED = true

/** How far the guide lines reach behind the rear (m), and how wide they're painted. */
const GUIDE_LENGTH = 6
const GUIDE_WIDTH = 0.12

export interface OurCamera {
  at: Vec
  height: number
  back: Vec
  right: Vec
}

export function ourCameraOf(t: Telemetry): { camera: OurCamera; path: PathPoint[] } | null {
  const found = rigOf(t)
  if (!found) return null
  // A trailer's rear end a metre in (TRAILER_IN): the camera and the lines start at its bumper.
  const rig = found.trailer ? { ...found, trailer: { ...found.trailer, rearBack: found.trailer.rearBack - TRAILER_IN } } : found
  const rear = rearOf(rig)
  const back = backOf(rear.heading)
  const past = PAST_REAR
  return {
    camera: {
      at: [rear.at[0] + back[0] * past, rear.at[1] + back[1] * past],
      height: rig.trailer ? HEIGHT_TRAILER : HEIGHT_TRUCK,
      back,
      right: rightOf(rear.heading),
    },
    path: reversePath(rig, GUIDE_LENGTH),
  }
}

/**
 * A point on the ground in the picture (pixels, `width` x `height`), `null` behind or too near the camera: its
 * lens as the plugin's (WIDE across, the same focal length up and down: square pixels).
 */
export function project(camera: OurCamera, [x, z]: Vec, width: number, height: number): [number, number] | null {
  const d: Vec = [x - camera.at[0], z - camera.at[1]]
  const along = d[0] * camera.back[0] + d[1] * camera.back[1]
  const across = d[0] * camera.right[0] + d[1] * camera.right[1]
  const pitch = (PITCH * Math.PI) / 180
  // In the camera: ahead (depth), up, and right on the screen.
  const ahead = along * Math.cos(pitch) + camera.height * Math.sin(pitch)
  const up = along * Math.sin(pitch) - camera.height * Math.cos(pitch)
  if (ahead < 0.2) return null
  const focal = width / 2 / Math.tan((WIDE * Math.PI) / 360)
  const right = MIRRORED ? across : -across
  return [width / 2 + (right / ahead) * focal, height / 2 - (up / ahead) * focal]
}

/** The guide lines over our camera's picture on `canvas` (cleared first; nothing without a rig). */
export function drawOurGuides(canvas: HTMLCanvasElement, t: Telemetry | null | undefined) {
  const context = canvas.getContext('2d')
  if (!context) return
  const ratio = window.devicePixelRatio || 1
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) {
    canvas.width = Math.round(width * ratio)
    canvas.height = Math.round(height * ratio)
  }
  context.setTransform(ratio, 0, 0, ratio, 0, 0)
  context.clearRect(0, 0, width, height)
  const placed = t ? ourCameraOf(t) : null
  if (!placed || placed.path.length < 2) return
  const { camera, path } = placed
  const at = (point: Vec) => project(camera, point, width, height)
  const colourAt = (travelled: number) =>
    travelled < GUIDE_MARKS[0] ? '#ff3b30' : travelled < GUIDE_MARKS[1] ? '#ffcc00' : '#34c759'
  const strip = (corners: Vec[], colour: string) => {
    const seen = corners.map(at)
    if (seen.every((p) => p !== null)) quad(context, seen as [number, number][], colour)
  }
  context.shadowColor = 'rgba(0, 0, 0, 0.55)'
  context.shadowBlur = 3
  for (const side of ['left', 'right'] as const) {
    const inward = side === 'left' ? 1 : -1
    for (let step = 1; step < path.length; step++) {
      const [a, b] = [path[step - 1], path[step]]
      const inA = towards(a[side], a.middle, GUIDE_WIDTH * inward)
      const inB = towards(b[side], b.middle, GUIDE_WIDTH * inward)
      strip([a[side], b[side], inB, inA], colourAt(a.travelled))
    }
  }
  for (const mark of GUIDE_MARKS) {
    const point = pointAt(path, mark)
    const ahead = pointAt(path, mark + GUIDE_WIDTH)
    if (!point || !ahead) continue
    const reach = Math.min(0.6, Math.hypot(point.right[0] - point.left[0], point.right[1] - point.left[1]) / 4)
    for (const side of ['left', 'right'] as const) {
      strip(
        [point[side], towards(point[side], point.middle, reach), towards(ahead[side], ahead.middle, reach), ahead[side]],
        colourAt(mark - 0.01),
      )
    }
  }
}
