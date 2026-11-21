import type { Beeping } from './sensor'

/**
 * The parking sensors' beeps, made with Web Audio: a short soft-edged tone, the time between them and their
 * loudness as the sensors say (sensor.ts `beepingFor`), or one steady tone right up against something. Beeps are
 * set on the audio clock a little ahead, so they keep their rhythm whatever the page is doing; a change of rhythm
 * takes from the next beep, as in a real truck.
 */

/** The tone (Hz), each beep's length (s) and the soft edges of it (s). */
const PITCH = 2150
const BEEP = 0.075
const EDGE = 0.008
/** How far ahead beeps are set on the audio clock, and how often that's looked at (ms). */
const AHEAD = 0.12
const TICK_MS = 25
/** The loudest the sensors get (the page's own sound, to the PC's speakers). */
const VOLUME = 0.35

export class Beeper {
  private context: AudioContext | null = null
  /** The player's volume (Settings: 1 as they were made). */
  private volume = 1
  private output: GainNode | null = null
  private beeping: Beeping | null = null
  /** When the next beep is due on the audio clock, and when the last one was (s). */
  private next = 0
  private last = -Infinity
  /** The steady tone, while it holds. */
  private steady: { oscillator: OscillatorNode; gain: GainNode } | null = null
  private timer: ReturnType<typeof setInterval> | null = null

  /** Beeps so (`null`: silent). */
  set(beeping: Beeping | null) {
    const was = this.beeping
    this.beeping = beeping
    if (!beeping) {
      this.stopSteady()
      return
    }
    const context = this.start()
    if (!context) return
    if (beeping.every === 0) {
      this.startSteady(context, beeping.loudness)
      return
    }
    this.stopSteady()
    // From silence (or the steady tone): the first beep at once. Nearer, quicker: the next beep as soon as the
    // new rhythm has it after the last.
    if (!was || was.every === 0) this.next = Math.max(this.next, context.currentTime + 0.01)
    else this.next = Math.min(this.next, Math.max(context.currentTime, this.last + beeping.every / 1000))
  }

  /** The player's volume for the beeps (1: as they were made), from now. */
  setVolume(volume: number) {
    this.volume = volume
    if (this.context && this.output) this.output.gain.setTargetAtTime(VOLUME * volume, this.context.currentTime, 0.02)
  }

  /** One beep at once, at its loudest (Settings: to hear the volume chosen). */
  sample() {
    const context = this.start()
    if (!context) return
    this.beepAt(context, context.currentTime + 0.02, 1)
  }

  /** Stops for good, its audio let go of. */
  close() {
    this.beeping = null
    this.stopSteady()
    if (this.timer !== null) clearInterval(this.timer)
    this.timer = null
    void this.context?.close()
    this.context = null
    this.output = null
  }

  private start(): AudioContext | null {
    if (!this.context) {
      try {
        this.context = new AudioContext({ latencyHint: 'interactive' })
      } catch {
        return null
      }
      this.output = this.context.createGain()
      this.output.gain.value = VOLUME * this.volume
      this.output.connect(this.context.destination)
      this.timer = setInterval(() => this.tick(), TICK_MS)
    }
    if (this.context.state === 'suspended') void this.context.resume()
    return this.context
  }

  private tick() {
    const context = this.context
    const beeping = this.beeping
    if (!context || !beeping || beeping.every === 0) return
    while (this.next < context.currentTime + AHEAD) {
      this.next = Math.max(this.next, context.currentTime)
      this.beepAt(context, this.next, beeping.loudness)
      this.last = this.next
      this.next += beeping.every / 1000
    }
  }

  /** A tone through an envelope: two harmonics, for a buzzer's edge without its harshness. */
  private tone(context: AudioContext, gain: GainNode): OscillatorNode {
    const oscillator = context.createOscillator()
    const wave = context.createPeriodicWave(new Float32Array([0, 0, 0, 0]), new Float32Array([0, 1, 0, 0.18]))
    oscillator.setPeriodicWave(wave)
    oscillator.frequency.value = PITCH
    oscillator.connect(gain)
    gain.connect(this.output!)
    return oscillator
  }

  private beepAt(context: AudioContext, at: number, loudness: number) {
    const gain = context.createGain()
    gain.gain.setValueAtTime(0, at)
    gain.gain.linearRampToValueAtTime(loudness, at + EDGE)
    gain.gain.setValueAtTime(loudness, at + BEEP - EDGE)
    gain.gain.linearRampToValueAtTime(0, at + BEEP)
    const oscillator = this.tone(context, gain)
    oscillator.start(at)
    oscillator.stop(at + BEEP + 0.01)
    oscillator.onended = () => gain.disconnect()
  }

  private startSteady(context: AudioContext, loudness: number) {
    if (this.steady) {
      this.steady.gain.gain.setTargetAtTime(loudness, context.currentTime, 0.02)
      return
    }
    const gain = context.createGain()
    gain.gain.setValueAtTime(0, context.currentTime)
    gain.gain.linearRampToValueAtTime(loudness, context.currentTime + EDGE)
    const oscillator = this.tone(context, gain)
    oscillator.start()
    this.steady = { oscillator, gain }
  }

  private stopSteady() {
    const steady = this.steady
    const context = this.context
    if (!steady || !context) return
    this.steady = null
    const now = context.currentTime
    steady.gain.gain.cancelScheduledValues(now)
    steady.gain.gain.setValueAtTime(steady.gain.gain.value, now)
    steady.gain.gain.linearRampToValueAtTime(0, now + EDGE)
    steady.oscillator.stop(now + EDGE + 0.01)
    steady.oscillator.onended = () => steady.gain.disconnect()
    // Back to beeping: the next beep a beat after the tone ends.
    this.next = now + 0.15
  }
}

/** The page's one beeper: the camera's beeps, and the sample Settings plays. */
export const beeper = new Beeper()
