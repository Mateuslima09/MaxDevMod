import { wrap } from './shape'

/**
 * How far the phone is turned like a wheel (around the axis out of its screen) and tilted, from its motion sensor:
 * the gyroscope for speed (no lag), and gravity to keep it from drifting (a complementary filter). Degrees,
 * counter-clockwise as the screen faces the player; tilt forward (the top edge away) positive.
 */
export class Orientation {
  /** The turn and the tilt now (`null` until the first reading). */
  turn: number | null = null
  tilt: number | null = null
  /** Whether the phone has a gyroscope (else gravity alone, steadier with more smoothing). */
  gyroscope = false
  /** The gyroscope's sense against gravity's: found as the phone turns (phones don't all agree). */
  private sense = 1
  private agreement = 0
  private lastGravity: number | null = null
  private lastAt: number | null = null

  /**
   * A reading: gravity's pull as the phone measures it (m/s², device axes: x right, y up, z out of the screen; its
   * sign doesn't matter), and the turning speed around z (°/s, `null` without a gyroscope), at `now` (ms).
   */
  update(ax: number, ay: number, az: number, rateZ: number | null, now: number) {
    const flat = Math.hypot(ax, ay)
    if (!Number.isFinite(flat) || !Number.isFinite(az)) return
    const byGravity = (Math.atan2(ax, ay) * 180) / Math.PI
    const tilt = (Math.atan2(az, flat) * 180) / Math.PI
    const ms = this.lastAt === null ? 0 : Math.min(100, Math.max(0, now - this.lastAt))
    this.lastAt = now
    this.tilt = this.tilt === null ? tilt : this.tilt + (tilt - this.tilt) * 0.3
    if (rateZ !== null && Number.isFinite(rateZ)) this.gyroscope = true
    if (this.turn === null || this.lastGravity === null) {
      this.turn = byGravity
      this.lastGravity = byGravity
      return
    }
    // Gravity's say counts less as the phone lies flat (its turn then can't be told from gravity).
    const trust = Math.min(1, flat / 6)
    const gravityMoved = wrap(byGravity - this.lastGravity)
    this.lastGravity = byGravity
    if (this.gyroscope && rateZ !== null && Number.isFinite(rateZ)) {
      const moved = (rateZ * this.sense * ms) / 1000
      // Turning quickly, the gyroscope and gravity should agree on which way: if they keep disagreeing, the
      // phone's gyroscope counts the other way round.
      if (Math.abs(rateZ) > 30 && trust > 0.5) {
        this.agreement += Math.sign(moved) === Math.sign(gravityMoved) ? 1 : -1
        this.agreement = Math.max(-20, Math.min(20, this.agreement))
        if (this.agreement <= -12) {
          this.sense = -this.sense
          this.agreement = 0
        }
      }
      const predicted = this.turn + moved
      this.turn = wrap(predicted + wrap(byGravity - predicted) * 0.03 * trust)
    } else {
      this.turn = wrap(this.turn + wrap(byGravity - this.turn) * Math.max(0.05, trust))
    }
  }
}
