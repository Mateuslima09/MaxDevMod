import { describe, expect, it } from 'vitest'
import type { Telemetry } from '../../shared/host'
import { AWAY_AHEAD, LEAVE_AFTER_MS, ReverseLatch } from './shown'

/** The engine's gear and the dashboard's the same (a manual gearbox, or an automatic driving). */
const at = (gear: number, speed = 0, electricEnabled = true) =>
  ({ gear, displayedGear: gear, speed, electricEnabled }) as Telemetry

describe('ReverseLatch', () => {
  it('shows with reverse chosen on an automatic standing still, its engine in neutral, and stays', () => {
    // Owner 2026-11-13: it came only while accelerating and went as the throttle was let go.
    const latch = new ReverseLatch()
    const chosen = (gear: number) => ({ gear, displayedGear: -1, speed: 0, electricEnabled: true }) as Telemetry
    expect(latch.update(chosen(0), 0)).toBe(true)
    expect(latch.update(chosen(-1), 500)).toBe(true)
    expect(latch.update(chosen(0), 1000)).toBe(true)
    expect(latch.update(chosen(0), 1000 + LEAVE_AFTER_MS * 3)).toBe(true)
  })

  it('shows the moment reverse is put in, not before', () => {
    const latch = new ReverseLatch()
    expect(latch.update(at(0), 0)).toBe(false)
    expect(latch.update(at(3, 5), 100)).toBe(false)
    expect(latch.update(at(-1), 200)).toBe(true)
  })

  it(`holds ${LEAVE_AFTER_MS} ms in neutral, and stays going back into reverse`, () => {
    const latch = new ReverseLatch()
    latch.update(at(-1), 0)
    expect(latch.update(at(0), 1000)).toBe(true)
    expect(latch.update(at(0), 1000 + LEAVE_AFTER_MS - 1)).toBe(true)
    expect(latch.update(at(0), 1000 + LEAVE_AFTER_MS)).toBe(false)
    // Through neutral and back again within the moment: it never went.
    latch.update(at(-1), 5000)
    latch.update(at(0), 5100)
    expect(latch.update(at(-1), 5900)).toBe(true)
    expect(latch.update(at(0), 6000)).toBe(true)
    expect(latch.update(at(0), 6000 + LEAVE_AFTER_MS - 1)).toBe(true)
  })

  it('goes at once as the truck pulls away ahead', () => {
    const latch = new ReverseLatch()
    latch.update(at(-1), 0)
    expect(latch.update(at(1, 1), 100)).toBe(true)
    expect(latch.update(at(1, AWAY_AHEAD + 0.1), 200)).toBe(false)
  })

  it('goes with the electrics off or no game, even in reverse', () => {
    const latch = new ReverseLatch()
    latch.update(at(-1), 0)
    expect(latch.update(at(-1, 0, false), 100)).toBe(false)
    latch.update(at(-1), 200)
    expect(latch.update(null, 300)).toBe(false)
    // Out of reverse once gone, it doesn't come back by itself.
    expect(latch.update(at(0), 400)).toBe(false)
  })
})
