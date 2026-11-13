import type { Telemetry } from '../../shared/host'

/**
 * When the reversing camera shows: the moment reverse is put in (the truck's electrics on), as a real one comes
 * up, and until a moment after it's taken out (going through neutral back into reverse, it stays), or at once if
 * the truck pulls away ahead or its electrics go off.
 */

/** How long out of reverse before the camera goes (ms), and how fast ahead it goes at once (m/s). */
export const LEAVE_AFTER_MS = 1200
export const AWAY_AHEAD = 2.5

export class ReverseLatch {
  private shown = false
  private outSince: number | null = null

  /** Whether the camera shows now, with the telemetry as it is (`null`: no game). */
  update(telemetry: Telemetry | null, now: number): boolean {
    if (!telemetry || !telemetry.electricEnabled) return this.leave()
    if (telemetry.gear < 0) {
      this.outSince = null
      this.shown = true
      return true
    }
    if (!this.shown) return false
    if (telemetry.speed > AWAY_AHEAD) return this.leave()
    this.outSince ??= now
    if (now - this.outSince >= LEAVE_AFTER_MS) return this.leave()
    return true
  }

  private leave(): false {
    this.shown = false
    this.outSince = null
    return false
  }
}
