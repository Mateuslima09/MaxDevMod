import { describe, expect, it } from 'vitest'
import {
  bearingOf,
  cameraOf,
  JACKKNIFE,
  rearOf,
  reversePath,
  type Rig,
  TRAILER_CAMERA_HEIGHT,
  TRUCK_CAMERA_HEIGHT,
} from './geometry'

/** A truck facing north (its back is +z), its rear axle at the origin, the wheel turned `steer`. */
const truck = (steer = 0): Rig => ({
  axle: [0, 0],
  heading: 0,
  steer,
  wheelbase: 3.9,
  hookBack: -0.3,
  rearBack: 1,
  halfWidth: 1.27,
  trailer: null,
})

/** The same truck with a trailer straight behind it. */
const withTrailer = (steer = 0): Rig => ({
  ...truck(steer),
  trailer: { heading: 0, axleBack: 9.2, rearBack: 12.8, halfWidth: 1.3 },
})

describe('reversePath', () => {
  it('goes straight back with the wheel straight', () => {
    const path = reversePath(truck(), 5)
    expect(path[0].middle).toEqual([0, 1])
    const last = path.at(-1)!
    expect(last.middle[0]).toBeCloseTo(0, 6)
    expect(last.middle[1]).toBeCloseTo(1 + last.travelled, 6)
    expect(last.travelled).toBeGreaterThanOrEqual(5)
    // The corners stay either side of the middle, the truck's width apart.
    expect(last.right[0] - last.left[0]).toBeCloseTo(2 * 1.27, 6)
  })

  it('takes the rear to the left with the wheel turned left (left of a truck facing north is -x)', () => {
    const last = reversePath(truck(0.3), 6).at(-1)!
    expect(last.middle[0]).toBeLessThan(-0.5)
    const right = reversePath(truck(-0.3), 6).at(-1)!
    expect(right.middle[0]).toBeCloseTo(-last.middle[0], 6)
  })

  it("takes a trailer's rear the other way, and stops before the rig folds", () => {
    const path = reversePath(withTrailer(0.3), 25)
    const last = path.at(-1)!
    expect(path[0].middle).toEqual([0, -0.3 + 12.8])
    // The truck's rear would go left: the trailer's goes right.
    expect(last.middle[0]).toBeGreaterThan(0.5)
    // Turned hard all the way, it folds before the 25 m: the path ends there, just past the jackknife angle.
    expect(last.travelled).toBeLessThan(25)
    expect(Math.abs(last.articulation)).toBeGreaterThan(JACKKNIFE)
    expect(path.slice(0, -1).every((point) => Math.abs(point.articulation) <= JACKKNIFE)).toBe(true)
  })

  it('keeps a trailer straight behind as it backs straight', () => {
    const last = reversePath(withTrailer(), 8).at(-1)!
    expect(last.articulation).toBeCloseTo(0, 6)
    expect(last.middle[0]).toBeCloseTo(0, 6)
  })
})

describe('the rear and the camera', () => {
  it("is the truck's own rear alone, the trailer's when coupled", () => {
    expect(rearOf(truck())).toEqual({ at: [0, 1], heading: 0, halfWidth: 1.27 })
    expect(rearOf(withTrailer())).toEqual({ at: [0, 12.5], heading: 0, halfWidth: 1.3 })
  })

  it('looks back from over the rear, higher on a trailer', () => {
    const onTruck = cameraOf(truck())
    expect(onTruck.height).toBe(TRUCK_CAMERA_HEIGHT)
    expect(Math.abs(onTruck.facing)).toBeCloseTo(Math.PI, 6)
    expect(cameraOf(withTrailer()).height).toBe(TRAILER_CAMERA_HEIGHT)
  })
})

describe('bearingOf', () => {
  it("turns a heading (counterclockwise, radians) into MapLibre's bearing (clockwise, degrees)", () => {
    expect(bearingOf(0)).toBe(0)
    expect(bearingOf(Math.PI / 2)).toBeCloseTo(270, 9)
    expect(bearingOf(-Math.PI / 2)).toBeCloseTo(90, 9)
    expect(bearingOf(Math.PI)).toBeCloseTo(180, 9)
    expect(bearingOf(4 * Math.PI)).toBeCloseTo(0, 9)
  })
})
