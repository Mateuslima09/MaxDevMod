import { type PointerEvent, useEffect, useRef, useState } from 'react'
import { postToHost } from '../shared/host'

/**
 * The coupling's steps by hand (owner 2026-11-19: "a drawing of the fifth wheel, the mouse on its lock: pull it to
 * uncouple, push it to couple; and the trailer's legs, cranked down and up as in real life"). The player knelt at
 * the truck's side or stood by the trailer's legs (the plugin's `freight`), and the host shows this large, with the
 * mouse, as it shows the cargo's papers. Done, it's answered as a paper collected; closed, nothing is done.
 *
 * - The lock (`FifthWheel`): its handle dragged out (pull) or in (push); let go before the end, it springs back.
 * - The legs (`LegCrank`): the crank turned round and round, [`TURNS`] turns, clockwise down, the other way up.
 */

export type Step = 'lockPull' | 'lockPush' | 'legsUp' | 'legsDown'

/** How far the handle moves (drawing units) and from what share of it the lock gives. */
const TRAVEL = 120
const GIVES_AT = 0.93
/** After a step is done, how long before the person gets up (ms). */
const DONE_AFTER = 900
/** The crank's turns to take the legs all the way. */
const TURNS = 4
/** The ratchet's clicks a turn. */
const CLICKS_PER_TURN = 8

let audio: AudioContext | null = null

function context(): AudioContext {
  audio ??= new AudioContext()
  if (audio.state === 'suspended') void audio.resume()
  return audio
}

/** A second of white noise, made once (the scrapes, the ratchet, the gears). */
let noiseBuffer: AudioBuffer | null = null
function noise(ctx: AudioContext): AudioBufferSourceNode {
  if (!noiseBuffer) {
    noiseBuffer = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate)
    const data = noiseBuffer.getChannelData(0)
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1
  }
  const source = ctx.createBufferSource()
  source.buffer = noiseBuffer
  source.loop = true
  return source
}

/** A burst of noise through a band (Hz, its Q), `loud` fading over `long` seconds, from `at`. */
function burst(ctx: AudioContext, at: number, band: number, q: number, loud: number, long: number): void {
  const source = noise(ctx)
  const filter = ctx.createBiquadFilter()
  filter.type = 'bandpass'
  filter.frequency.value = band
  filter.Q.value = q
  const gain = ctx.createGain()
  gain.gain.setValueAtTime(loud, at)
  gain.gain.exponentialRampToValueAtTime(0.0001, at + long)
  source.connect(filter).connect(gain).connect(ctx.destination)
  source.start(at, Math.random() * 0.5)
  source.stop(at + long + 0.02)
}

/** Steel ringing: its partials (Hz, loudness, seconds to fade), struck at `at`. */
function ring(ctx: AudioContext, at: number, partials: [number, number, number][]): void {
  for (const [frequency, loud, long] of partials) {
    const oscillator = ctx.createOscillator()
    oscillator.frequency.value = frequency
    const gain = ctx.createGain()
    gain.gain.setValueAtTime(0.0001, at)
    gain.gain.exponentialRampToValueAtTime(loud, at + 0.002)
    gain.gain.exponentialRampToValueAtTime(0.0001, at + long)
    oscillator.connect(gain).connect(ctx.destination)
    oscillator.start(at)
    oscillator.stop(at + long + 0.02)
  }
}

/** A heavy low knock: the weight of the steel (Hz falling over `long`). */
function thud(ctx: AudioContext, at: number, from: number, to: number, loud: number, long: number): void {
  const oscillator = ctx.createOscillator()
  oscillator.frequency.setValueAtTime(from, at)
  oscillator.frequency.exponentialRampToValueAtTime(to, at + long)
  const gain = ctx.createGain()
  gain.gain.setValueAtTime(0.0001, at)
  gain.gain.exponentialRampToValueAtTime(loud, at + 0.004)
  gain.gain.exponentialRampToValueAtTime(0.0001, at + long)
  oscillator.connect(gain).connect(ctx.destination)
  oscillator.start(at)
  oscillator.stop(at + long + 0.02)
}

/**
 * The owner's recordings (2026-11-19: "one for the uncoupling, one for the ratchet of the legs"), in
 * `public/sounds`: a sharp pop (`desengate.mp3`: 1 s, the pop from 0.11 to 0.3 s, silence after) and a ratchet
 * run (`catraca.mp3`: 19 clicks about 50 ms apart). Decoded once; played in pieces.
 */
const SAMPLES = { pop: '/sounds/desengate.mp3', ratchet: '/sounds/catraca.mp3' } as const
const decoded: Partial<Record<keyof typeof SAMPLES, AudioBuffer>> = {}
let loading = false

/** The recordings fetched and decoded, once (in the background). */
function load(ctx: AudioContext): void {
  if (loading) return
  loading = true
  for (const [name, url] of Object.entries(SAMPLES) as [keyof typeof SAMPLES, string][]) {
    void fetch(url)
      .then((response) => response.arrayBuffer())
      .then((bytes) => ctx.decodeAudioData(bytes))
      .then((buffer) => {
        decoded[name] = buffer
      })
      .catch(() => {
        // Not there: the made sound plays instead.
      })
  }
}

/** A piece of a recording (seconds from its start, how long), `loud`; false if it isn't decoded yet. */
function play(ctx: AudioContext, name: keyof typeof SAMPLES, from: number, long: number, loud: number): boolean {
  load(ctx)
  const buffer = decoded[name]
  if (!buffer) return false
  const source = ctx.createBufferSource()
  source.buffer = buffer
  const gain = ctx.createGain()
  const at = ctx.currentTime + 0.005
  gain.gain.setValueAtTime(loud, at)
  // A short fade at the end: no click cutting it off.
  gain.gain.setValueAtTime(loud, at + Math.max(0, long - 0.008))
  gain.gain.linearRampToValueAtTime(0, at + long)
  source.connect(gain).connect(ctx.destination)
  source.start(at, from, long)
  return true
}

/** The pop's piece of `desengate.mp3` (seconds). */
const POP = { from: 0.1, to: 0.42 }
/** Each click's start in `catraca.mp3` (seconds: a little before its peak, from its loudness every 5 ms), and how
 * long one is; played in turn, so the ratchet doesn't repeat one click. */
const CLICKS = [
  0.082, 0.122, 0.187, 0.232, 0.287, 0.332, 0.377, 0.427, 0.487, 0.542, 0.587, 0.647, 0.707, 0.782, 0.822, 0.862,
  0.912, 0.972,
]
const CLICK_LONG = 0.04
let nextClick = 0

/**
 * The steel's sounds, made here (owner 2026-11-19: "the sound is very odd, not like uncoupling; make one of
 * something turning too"): the lock giving (`clack`: a scrape, the jaw's steel ringing, its weight knocking), the
 * handle sliding back (`slide`: steel scraping), the crank's ratchet (`tick`: a pawl's click on a gear).
 */
function sound(kind: 'clack' | 'slide' | 'tick'): void {
  try {
    const ctx = context()
    const at = ctx.currentTime + 0.01
    // The owner's own recordings first (2026-11-19), the made ones while they load or if they can't.
    if (kind === 'clack' && play(ctx, 'pop', POP.from, POP.to - POP.from, 1)) return
    if (kind === 'tick') {
      const start = CLICKS[nextClick++ % CLICKS.length]
      if (play(ctx, 'ratchet', start, CLICK_LONG, 0.8)) return
    }
    if (kind === 'clack') {
      burst(ctx, at, 2400, 1.2, 0.5, 0.05)
      thud(ctx, at, 95, 42, 0.7, 0.22)
      ring(ctx, at + 0.004, [
        [312, 0.16, 0.55],
        [743, 0.1, 0.4],
        [1187, 0.07, 0.3],
        [1931, 0.05, 0.22],
        [2876, 0.03, 0.15],
      ])
      // The jaw's second half closing a moment after.
      burst(ctx, at + 0.07, 1800, 1.5, 0.25, 0.04)
      ring(ctx, at + 0.07, [
        [521, 0.06, 0.3],
        [1402, 0.03, 0.18],
      ])
    } else if (kind === 'tick') {
      burst(ctx, at, 3200, 2.5, 0.35, 0.025)
      ring(ctx, at, [
        [2650, 0.05, 0.06],
        [4100, 0.025, 0.04],
      ])
      thud(ctx, at, 180, 120, 0.12, 0.04)
    } else {
      const source = noise(ctx)
      const filter = ctx.createBiquadFilter()
      filter.type = 'bandpass'
      filter.Q.value = 3
      filter.frequency.setValueAtTime(1400, at)
      filter.frequency.exponentialRampToValueAtTime(700, at + 0.14)
      const gain = ctx.createGain()
      gain.gain.setValueAtTime(0.0001, at)
      gain.gain.exponentialRampToValueAtTime(0.18, at + 0.02)
      gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.16)
      source.connect(filter).connect(gain).connect(ctx.destination)
      source.start(at)
      source.stop(at + 0.2)
      thud(ctx, at + 0.15, 120, 70, 0.3, 0.1)
    }
  } catch {
    // No sound here: the drawing still shows.
  }
}

/**
 * The crank's gears turning: a low grinding, as loud as the crank turns fast, while it's held (`start`, `speed`
 * in turns a second, `stop`).
 */
class Gears {
  private source: AudioBufferSourceNode | null = null
  private gain: GainNode | null = null
  private filter: BiquadFilterNode | null = null

  start(): void {
    try {
      const ctx = context()
      this.source = noise(ctx)
      this.filter = ctx.createBiquadFilter()
      this.filter.type = 'bandpass'
      this.filter.Q.value = 4
      this.filter.frequency.value = 220
      this.gain = ctx.createGain()
      this.gain.gain.value = 0
      this.source.connect(this.filter).connect(this.gain).connect(ctx.destination)
      this.source.start()
    } catch {
      this.source = null
    }
  }

  speed(turns: number): void {
    if (!this.gain || !this.filter || !audio) return
    const now = audio.currentTime
    const fast = Math.min(1, turns / 1.5)
    this.gain.gain.setTargetAtTime(fast * 0.1, now, 0.04)
    this.filter.frequency.setTargetAtTime(160 + fast * 260, now, 0.06)
  }

  stop(): void {
    if (this.gain && audio) this.gain.gain.setTargetAtTime(0, audio.currentTime, 0.05)
    const source = this.source
    if (source && audio) source.stop(audio.currentTime + 0.3)
    this.source = null
  }
}

interface StepWords {
  title: string
  hint: string
  done: string
}

const WORDS: Record<'en' | 'pt', Record<Step, StepWords> & { close: string }> = {
  en: {
    lockPull: {
      title: 'Fifth wheel',
      hint: 'Grab the red handle and pull it all the way out to open the lock',
      done: 'Lock open: the trailer is free',
    },
    lockPush: {
      title: 'Fifth wheel',
      hint: 'Grab the red handle and push it all the way in to lock the trailer’s pin',
      done: 'Locked: now raise the trailer’s legs',
    },
    legsDown: {
      title: 'Trailer legs',
      hint: 'Grab the crank’s knob and turn it clockwise until the feet are on the ground',
      done: 'Legs down: the trailer stands on its own',
    },
    legsUp: {
      title: 'Trailer legs',
      hint: 'Grab the crank’s knob and turn it counter-clockwise until the legs are up',
      done: 'Legs up',
    },
    close: 'Leave it',
  },
  pt: {
    lockPull: {
      title: 'Quinta roda',
      hint: 'Segure o puxador vermelho e puxe até o fim para abrir a trava',
      done: 'Trava aberta: a carreta está solta',
    },
    lockPush: {
      title: 'Quinta roda',
      hint: 'Segure o puxador vermelho e empurre até o fim para travar o pino da carreta',
      done: 'Travada: agora suba os pés da carreta',
    },
    legsDown: {
      title: 'Pés da carreta',
      hint: 'Segure a manopla da manivela e gire no sentido horário até os pés tocarem o chão',
      done: 'Pés no chão: a carreta se apoia sozinha',
    },
    legsUp: {
      title: 'Pés da carreta',
      hint: 'Segure a manopla da manivela e gire no sentido anti-horário até os pés subirem',
      done: 'Pés levantados',
    },
    close: 'Deixar',
  },
}

/** The step's answer, a moment after it's done (the sound and the green first). */
function useDone(done: boolean): void {
  useEffect(() => {
    if (!done) return
    sound('clack')
    const timer = setTimeout(() => postToHost({ type: 'paperDone', collected: true }), DONE_AFTER)
    return () => clearTimeout(timer)
  }, [done])
}

/** The step in the paper's place: its title, its drawing, what to do, and its close. */
export function CouplingStep({ step, language }: { step: Step; language: 'en' | 'pt' }) {
  const words = WORDS[language]
  const [done, setDone] = useState(false)
  useDone(done)
  // The recordings ready before the first click.
  useEffect(() => {
    try {
      load(context())
    } catch {
      // No sound here.
    }
  }, [])
  return (
    <article className="fifth-wheel">
      <button
        type="button"
        className="paper-close"
        aria-label={words.close}
        title={words.close}
        onClick={() => postToHost({ type: 'paperDone', collected: false })}
      >
        ×
      </button>
      <h1>{words[step].title}</h1>
      {step === 'lockPull' || step === 'lockPush' ? (
        <FifthWheel push={step === 'lockPush'} done={done} onDone={() => setDone(true)} />
      ) : (
        <LegCrank up={step === 'legsUp'} done={done} onDone={() => setDone(true)} />
      )}
      <p className={`fifth-wheel-hint${done ? ' released' : ''}`}>{done ? words[step].done : words[step].hint}</p>
    </article>
  )
}

/** A pointer's place in the drawing's own units. */
function inDrawing(svg: SVGSVGElement | null, event: PointerEvent): { x: number; y: number } | null {
  const matrix = svg?.getScreenCTM()
  if (!svg || !matrix) return null
  const point = svg.createSVGPoint()
  point.x = event.clientX
  point.y = event.clientY
  const at = point.matrixTransform(matrix.inverse())
  return { x: at.x, y: at.y }
}

/** The fifth wheel from above: its handle out (open) or in (locked), dragged the way the step goes. */
function FifthWheel({ push, done, onDone }: { push: boolean; done: boolean; onDone: () => void }) {
  // How far out the handle is (0 in, 1 out): pulling starts in, pushing starts out.
  const rest = push ? 1 : 0
  const [out, setOut] = useState(rest)
  const drag = useRef<{ from: number; at: number } | null>(null)
  const svg = useRef<SVGSVGElement>(null)

  const start = (event: PointerEvent<SVGGElement>) => {
    const at = inDrawing(svg.current, event)
    if (done || !at) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = { from: at.x, at: out }
  }
  const move = (event: PointerEvent<SVGGElement>) => {
    const d = drag.current
    const at = inDrawing(svg.current, event)
    if (!d || done || !at) return
    const now = Math.max(0, Math.min(1, d.at + (at.x - d.from) / TRAVEL))
    setOut(now)
    const gone = push ? 1 - now : now
    if (gone >= GIVES_AT) {
      setOut(push ? 0 : 1)
      drag.current = null
      onDone()
    }
  }
  const end = () => {
    if (!drag.current) return
    drag.current = null
    if (!done) {
      if (Math.abs(out - rest) > 0.05) sound('slide')
      setOut(rest)
    }
  }

  const shift = out * TRAVEL
  // The jaw: open with the handle out (38°), shut with it in.
  const jaw = Math.min(1, out / GIVES_AT) * 38
  const progress = push ? 1 - out : out
  const colour = done ? '#30d158' : '#e5302a'

  return (
    <svg ref={svg} viewBox="0 0 640 420" className="fifth-wheel-drawing" role="img">
      <defs>
        <linearGradient id="fw-plate" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#5b6068" />
          <stop offset="1" stopColor="#2f3237" />
        </linearGradient>
        <radialGradient id="fw-grease" cx="0.5" cy="0.45" r="0.6">
          <stop offset="0" stopColor="#1b1c1f" stopOpacity="0.9" />
          <stop offset="1" stopColor="#1b1c1f" stopOpacity="0" />
        </radialGradient>
      </defs>
      {/* The truck's frame rails under it. */}
      <rect x="150" y="40" width="34" height="360" rx="4" fill="#23252a" />
      <rect x="376" y="40" width="34" height="360" rx="4" fill="#23252a" />
      {/* The plate: round in front, its ramps open at the back (the bottom), where the trailer's pin comes in. */}
      <path
        d="M280 50 a150 150 0 0 1 150 150 v120 l-60 60 h-40 l-38 -150 h-24 l-38 150 h-40 l-60 -60 v-120 a150 150 0 0 1 150 -150 Z"
        fill="url(#fw-plate)"
        stroke="#7d828b"
        strokeWidth="3"
      />
      <ellipse cx="280" cy="200" rx="120" ry="110" fill="url(#fw-grease)" />
      {/* The trailer's pin in the jaw, and the jaw, open with the handle out. */}
      <circle cx="280" cy="200" r="18" fill="#111" stroke="#9aa0a8" strokeWidth="3" />
      <g transform={`rotate(${-jaw} 252 214)`}>
        <path d="M252 214 a30 30 0 0 1 28 -42 v12 a18 18 0 0 0 -16 30 Z" fill="#c9ccd1" />
      </g>
      <g transform={`rotate(${jaw} 308 214)`}>
        <path d="M308 214 a30 30 0 0 0 -28 -42 v12 a18 18 0 0 1 16 30 Z" fill="#c9ccd1" />
      </g>
      {/* The lock's rod, from the jaw out of the plate's side, and the handle at its end. */}
      <rect x={300} y={232} width={150 + shift} height={10} rx={5} fill="#9aa0a8" />
      <g
        className={`fifth-wheel-handle${done ? ' released' : ''}`}
        transform={`translate(${shift} 0)`}
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={end}
      >
        <rect x="440" y="214" width="16" height="46" rx="4" fill={colour} />
        <rect x="452" y="222" width="44" height="30" rx="10" fill={colour} />
        <rect x="460" y="230" width="28" height="14" rx="6" fill="rgb(0 0 0 / 0.25)" />
        {/* A wider grip than the handle drawn, easy to catch with the mouse's dot. */}
        <rect x="420" y="190" width="110" height="94" fill="transparent" />
      </g>
      {/* Which way, and how far. */}
      {!done && (
        <path
          d={push ? 'M600 160 h-70 m14 -12 l-14 12 l14 12' : 'M530 160 h70 m-14 -12 l14 12 l-14 12'}
          fill="none"
          stroke="rgb(255 255 255 / 0.5)"
          strokeWidth="4"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      <rect x="440" y="320" width={TRAVEL + 56} height="8" rx="4" fill="rgb(255 255 255 / 0.12)" />
      <rect x="440" y="320" width={(TRAVEL + 56) * progress} height="8" rx="4" fill={done ? '#30d158' : '#ff9f0a'} />
    </svg>
  )
}

/** The trailer's leg from its side, its crank turned round: the inner leg comes down (or goes up) as it turns. */
function LegCrank({ up, done, onDone }: { up: boolean; done: boolean; onDone: () => void }) {
  // How far it's gone (0..1) and the crank's angle (radians, drawn).
  const [progress, setProgress] = useState(0)
  const [angle, setAngle] = useState(-Math.PI / 2)
  const drag = useRef<{ last: number; turned: number; ticks: number; time: number } | null>(null)
  const svg = useRef<SVGSVGElement>(null)
  const gears = useRef(new Gears())
  useEffect(() => () => gears.current.stop(), [])
  const centre = { x: 430, y: 140 }
  const arm = 62

  const angleAt = (event: PointerEvent) => {
    const at = inDrawing(svg.current, event)
    return at ? Math.atan2(at.y - centre.y, at.x - centre.x) : null
  }
  const start = (event: PointerEvent<SVGGElement>) => {
    const a = angleAt(event)
    if (done || a === null) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = {
      last: a,
      turned: progress * TURNS * 2 * Math.PI,
      ticks: Math.floor(progress * TURNS * CLICKS_PER_TURN),
      time: performance.now(),
    }
    gears.current.start()
  }
  const move = (event: PointerEvent<SVGGElement>) => {
    const d = drag.current
    const a = angleAt(event)
    if (!d || done || a === null) return
    let delta = a - d.last
    if (delta > Math.PI) delta -= 2 * Math.PI
    if (delta < -Math.PI) delta += 2 * Math.PI
    d.last = a
    setAngle(a)
    // The gears as loud as the crank turns fast (turns a second).
    const now = performance.now()
    const dt = Math.max(1, now - d.time) / 1000
    d.time = now
    gears.current.speed(Math.abs(delta) / (2 * Math.PI) / dt)
    // On the screen, y down: an angle growing turns clockwise. Down: clockwise; up: the other way.
    const way = up ? -delta : delta
    d.turned = Math.max(0, Math.min(TURNS * 2 * Math.PI, d.turned + way))
    const share = d.turned / (TURNS * 2 * Math.PI)
    setProgress(share)
    const ticks = Math.floor(share * TURNS * CLICKS_PER_TURN)
    if (ticks > d.ticks) sound('tick')
    d.ticks = ticks
    if (share >= 0.999) {
      drag.current = null
      gears.current.stop()
      onDone()
    }
  }
  const end = () => {
    drag.current = null
    gears.current.stop()
  }

  // The inner leg out of the outer one: up 30, down 130 (its foot on the ground at 130).
  const down = up ? 1 - progress : progress
  const inner = 30 + down * 100
  const knob = { x: centre.x + Math.cos(angle) * arm, y: centre.y + Math.sin(angle) * arm }

  return (
    <svg ref={svg} viewBox="0 0 640 420" className="fifth-wheel-drawing" role="img">
      {/* The trailer's underside above, the ground below. */}
      <rect x="40" y="30" width="560" height="34" rx="4" fill="#3a3d43" />
      <rect x="40" y="380" width="560" height="10" fill="#4a3f33" />
      {/* The outer leg, the gearbox and its shaft. */}
      <rect x="250" y="64" width="54" height="180" rx="4" fill="#5b6068" stroke="#7d828b" strokeWidth="2" />
      <rect x="236" y="110" width="82" height="60" rx="8" fill="#44484f" stroke="#7d828b" strokeWidth="2" />
      <rect x="318" y="134" width={centre.x - 318} height="12" rx="6" fill="#9aa0a8" />
      {/* The inner leg and its foot. */}
      <rect x="262" y={244} width="30" height={inner} fill="#8c9199" />
      <rect x="240" y={244 + inner} width="74" height="12" rx="3" fill="#2d2f33" />
      {/* The crank: its hub, its arm to the knob. */}
      <circle cx={centre.x} cy={centre.y} r={arm} fill="none" stroke="rgb(255 255 255 / 0.1)" strokeWidth="2" strokeDasharray="6 8" />
      <line x1={centre.x} y1={centre.y} x2={knob.x} y2={knob.y} stroke="#b9bdc4" strokeWidth="10" strokeLinecap="round" />
      <circle cx={centre.x} cy={centre.y} r="12" fill="#6b7078" />
      <g
        className={`fifth-wheel-handle${done ? ' released' : ''}`}
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={end}
      >
        <circle cx={knob.x} cy={knob.y} r="18" fill={done ? '#30d158' : '#e5302a'} />
        {/* The whole ring the knob goes round: grabbed anywhere on it. */}
        <circle cx={centre.x} cy={centre.y} r={arm + 34} fill="transparent" />
      </g>
      {/* Which way to turn. */}
      {!done && (
        <path
          d={
            up
              ? `M${centre.x + 96} ${centre.y + 20} a96 96 0 0 0 -40 -86 m-2 18 l2 -18 l18 6`
              : `M${centre.x + 96} ${centre.y - 20} a96 96 0 0 1 -40 86 m-2 -18 l2 18 l18 -6`
          }
          fill="none"
          stroke="rgb(255 255 255 / 0.5)"
          strokeWidth="4"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      <rect x="380" y="320" width="200" height="8" rx="4" fill="rgb(255 255 255 / 0.12)" />
      <rect x="380" y="320" width={200 * progress} height="8" rx="4" fill={done ? '#30d158' : '#ff9f0a'} />
    </svg>
  )
}
