import { describe, expect, it } from 'vitest'
import { MAP_FPS, mapFrameDue } from './frameCap'

/** The display's frames over a second at `hz` (each a little early or late by `waver`, ms, in turn), and the
 * gaps between those the map is drawn on. */
function drawn(hz: number, waver = 0): number[] {
  let due = 0
  const at: number[] = []
  for (let tick = 0; tick < hz; tick++) {
    const now = (tick * 1000) / hz + (tick % 2 ? waver : -waver)
    const next = mapFrameDue(now, due)
    if (next === null) continue
    due = next
    at.push(now)
  }
  return at.slice(1).map((time, i) => Math.round(time - at[i]))
}

describe('the map drawn at most 30 a second', () => {
  it('is 30 a second', () => {
    expect(MAP_FPS).toBe(30)
  })

  it('draws every 2nd frame of a 60 Hz display, wavering or not', () => {
    expect(drawn(60).length + 1).toBe(30)
    expect(new Set(drawn(60))).toEqual(new Set([33]))
    expect(drawn(60, 1).length + 1).toBe(30)
  })

  it('draws 2 of 5 frames at 75 Hz', () => {
    const gaps = drawn(75)
    expect(gaps.length + 1).toBe(30)
    expect(new Set(gaps)).toEqual(new Set([27, 40]))
  })

  it("draws 30 of a 144 Hz display's, every 4th or 5th", () => {
    const gaps = drawn(144)
    expect(gaps.length + 1).toBe(30)
    expect(new Set(gaps)).toEqual(new Set([28, 35]))
  })

  it('draws every frame of a 30 Hz display', () => {
    expect(drawn(30).length + 1).toBe(30)
  })

  it('counts on from now after a pause', () => {
    expect(mapFrameDue(5000, 100)).toBeCloseTo(5000 + 1000 / 30)
  })
})
