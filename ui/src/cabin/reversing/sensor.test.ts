import { describe, expect, it } from 'vitest'
import type { Rear } from './geometry'
import { BEEP_FROM, beepingFor, nearestOf, type Obstacle, readSensors, STEADY_AT, withRays, zoneOf } from './sensor'

/** A rear at the origin facing north: its back is +z, its right +x. */
const rear: Rear = { at: [0, 0], heading: 0, halfWidth: 1.27 }

/** A wall across behind the rear, `gap` metres off. */
const wall = (gap: number): Obstacle => ({
  kind: 'outline',
  ring: [
    [-10, gap],
    [10, gap],
    [10, gap + 0.5],
    [-10, gap + 0.5],
  ],
})

describe('readSensors', () => {
  it('sees a wall 1 m back at about 1 m with the middle sensors', () => {
    const readings = readSensors(rear, [wall(1)])
    expect(readings[1]).toBeCloseTo(1, 6)
    expect(readings[2]).toBeCloseTo(1, 6)
    // The corners see it too, looking out a little: not nearer than it is.
    expect(readings[0]).not.toBeNull()
    expect(readings[0]!).toBeGreaterThanOrEqual(1)
    expect(nearestOf(readings)).toBeCloseTo(1, 6)
  })

  it('sees nothing beyond their range, or with nothing round', () => {
    expect(readSensors(rear, [wall(3.5)])).toEqual([null, null, null, null])
    expect(readSensors(rear, [])).toEqual([null, null, null, null])
    // Ahead of the rear is not behind it.
    expect(readSensors(rear, [wall(-2)])).toEqual([null, null, null, null])
  })

  it('sees a post at the right corner with the right-hand sensors only', () => {
    const readings = readSensors(rear, [{ kind: 'round', at: [1.6, 1], radius: 0.2 }])
    expect(readings[0]).toBeNull()
    expect(readings[3]).not.toBeNull()
    expect(readings[3]!).toBeLessThan(1.2)
  })

  it('turns with the rear', () => {
    // Facing east (a quarter turn left of north): its back is +x... the wall 1 m along +x.
    const turned: Rear = { at: [0, 0], heading: Math.PI / 2, halfWidth: 1.27 }
    const east: Obstacle = {
      kind: 'outline',
      ring: [
        [1, -10],
        [1, 10],
        [1.5, 10],
        [1.5, -10],
      ],
    }
    expect(nearestOf(readSensors(turned, [east]))).toBeCloseTo(1, 6)
  })
})

describe('withRays', () => {
  const map = [null, 2.5, 1.2, null]
  it("takes the game's physics' readings where they're nearer or the map has none, while the rays work", () => {
    expect(withRays(map, { rays: 1, raySensors: [1.8, 0.7, -1, -1] })).toEqual([1.8, 0.7, 1.2, null])
  })
  it("keeps the map's alone otherwise", () => {
    expect(withRays(map, { rays: 0, raySensors: [0.5, 0.5, 0.5, 0.5] })).toEqual(map)
    expect(withRays(map, { rays: 2, raySensors: [0.5, 0.5, 0.5, 0.5] })).toEqual(map)
    // A plugin from before the rays: no such fields.
    expect(withRays(map, {} as never)).toEqual(map)
  })
})

describe('nearestOf', () => {
  it('is the nearest reading, or none', () => {
    expect(nearestOf([null, 2.1, 0.9, null])).toBe(0.9)
    expect(nearestOf([null, null])).toBeNull()
  })
})

describe('beepingFor', () => {
  it('is silent with nothing near, or beyond where it starts', () => {
    expect(beepingFor(null)).toBeNull()
    expect(beepingFor(BEEP_FROM + 0.01)).toBeNull()
  })

  it('beeps slowly and quietly far off, a steady loud tone right up against it', () => {
    expect(beepingFor(BEEP_FROM)).toEqual({ every: 950, loudness: 0.35 })
    expect(beepingFor(STEADY_AT)).toEqual({ every: 0, loudness: 1 })
    expect(beepingFor(0)).toEqual({ every: 0, loudness: 1 })
  })

  it('beeps quicker and louder as it nears', () => {
    let before = beepingFor(BEEP_FROM)!
    for (let distance = BEEP_FROM - 0.25; distance > STEADY_AT; distance -= 0.25) {
      const now = beepingFor(distance)!
      expect(now.every).toBeLessThan(before.every)
      expect(now.loudness).toBeGreaterThan(before.loudness)
      before = now
    }
    // Just short of the steady tone: as quick as the beeps get.
    expect(beepingFor(STEADY_AT + 0.001)!.every).toBe(110)
  })
})

describe('zoneOf', () => {
  it('is red near, yellow, then green', () => {
    expect(zoneOf(0.5)).toBe('red')
    expect(zoneOf(1.2)).toBe('yellow')
    expect(zoneOf(2.5)).toBe('green')
  })
})
