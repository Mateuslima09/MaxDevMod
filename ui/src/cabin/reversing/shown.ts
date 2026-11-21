import type { Telemetry } from '../../shared/host'

/**
 * When the reversing camera shows: the moment reverse is put in (the truck's electrics on), as a real one comes
 * up, and until a moment after it's taken out (going through neutral back into reverse, it stays), or at once if
 * the truck pulls away ahead or its electrics go off.
 */

/** How long out of reverse before the camera goes (ms), and how fast ahead it goes at once (m/s). */
export const LEAVE_AFTER_MS = 1200
export const AWAY_AHEAD = 2.5

/**
 * Whether the truck is in reverse: the gear on the dashboard (what the player put in), or the one the engine has.
 * Stopped with an automatic gearbox, the engine's is neutral (0) though reverse is chosen, and it only engages as
 * the truck is driven (owner 2026-11-13: the camera came only while accelerating). The host sends
 * the telemetry more often by the same test (crates/host/src/lib.rs, `TELEMETRY_REVERSING_MS`).
 */
export function inReverse(telemetry: Pick<Telemetry, 'gear' | 'displayedGear'>): boolean {
  return telemetry.displayedGear < 0 || telemetry.gear < 0
}

export class ReverseLatch {
  private shown = false
  private outSince: number | null = null

  /** Whether the camera shows now, with the telemetry as it is (`null`: no game). */
  update(telemetry: Telemetry | null, now: number): boolean {
    if (!telemetry || !telemetry.electricEnabled) return this.leave()
    if (inReverse(telemetry)) {
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
