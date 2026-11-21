import type { PathPoint, Vec } from './geometry'

/** The guide lines' marks across, metres behind the rear (red, yellow, green up to them). Both cameras'. */
export const GUIDE_MARKS = [1, 2, 3]

/** `from` moved `distance` metres towards `to`. */
export function towards(from: Vec, to: Vec, distance: number): Vec {
  const length = Math.hypot(to[0] - from[0], to[1] - from[1]) || 1
  return [from[0] + ((to[0] - from[0]) / length) * distance, from[1] + ((to[1] - from[1]) / length) * distance]
}

/** The path's point `travelled` metres back (between its steps), `null` past its end. */
export function pointAt(path: PathPoint[], travelled: number): PathPoint | null {
  for (let at = 1; at < path.length; at++) {
    const [a, b] = [path[at - 1], path[at]]
    if (b.travelled < travelled) continue
    const k = b.travelled > a.travelled ? (travelled - a.travelled) / (b.travelled - a.travelled) : 0
    const mix = (p: Vec, q: Vec): Vec => [p[0] + (q[0] - p[0]) * k, p[1] + (q[1] - p[1]) * k]
    return {
      left: mix(a.left, b.left),
      middle: mix(a.middle, b.middle),
      right: mix(a.right, b.right),
      travelled,
      articulation: a.articulation,
    }
  }
  return null
}

export function quad(context: CanvasRenderingContext2D, corners: (readonly [number, number])[], colour: string) {
  context.fillStyle = colour
  context.beginPath()
  context.moveTo(corners[0][0], corners[0][1])
  for (const [x, y] of corners.slice(1)) context.lineTo(x, y)
  context.closePath()
  context.fill()
}
