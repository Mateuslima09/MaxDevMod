import { type ReactNode, useEffect, useRef, useState } from 'react'
import { useCharacterSwitches, useLive } from '../cabin/state/store'
import { postToHost } from '../shared/host'
import { List, Row, Segmented } from './ios'
import { RowIcon } from './icons'

/**
 * The suspension (owner 2026-11-18: "an app to control the truck's suspension, and the trailer's, an elaborate one,
 * with animation"): the rig from the side, its body riding on its air springs as high as the game says they are, and
 * the game's own controls (`westplay_shared::TRUCK_COMMANDS`) held from the phone. The heights are the springs' (the
 * SDK's `wheel.suspension.deflection`), from where they were when the app first read them or was last levelled.
 * The game's own limits hold: it raises and lowers as far as the truck's air suspension goes.
 */

/** What the buttons move: the truck as a whole, its front, its rear, the trailer. */
type Part = 'all' | 'front' | 'rear' | 'trailer'

const COMMANDS: Record<Part, { up: string[]; down: string[] }> = {
  all: { up: ['frontsuspup', 'rearsuspup'], down: ['frontsuspdwn', 'rearsuspdwn'] },
  front: { up: ['frontsuspup'], down: ['frontsuspdwn'] },
  rear: { up: ['rearsuspup'], down: ['rearsuspdwn'] },
  trailer: { up: ['trlrsuspup'], down: ['trlrsuspdwn'] },
}

/** How long the presets hold their buttons (ms): long enough to reach the game's limit. */
const PRESET_MS = 5000
/** After levelling, the heights are taken as the new 0 once it has settled (ms). */
const SETTLE_MS = 2600
/** Pixels of the drawing per metre the springs move (exaggerated, so a few centimetres show), and how far at most. */
const PX_PER_M = 260
const MOST_PX = 26
/** Centimetres the gauges show either side of 0. */
const GAUGE_CM = 12

/** Where the springs were taken as 0 (front, rear, trailer; metres): kept while the phone's page lives. */
let zero: [number | null, number | null, number | null] = [null, null, null]

/** How long the trailer stays in the drawing after the game stops saying it's coupled (ms): a moment without it
 * (a frame's telemetry, its springs not read) made the drawing go back and forth (owner 2026-11-18). */
const TRAILER_GONE_MS = 2000

/** Whether a trailer is coupled, steadily: at once as it is, no more only once it hasn't been for a while. */
function useCoupled(): boolean {
  const connected = useLive((state) => state.telemetry?.trailerConnected ?? false)
  const [coupled, setCoupled] = useState(connected)
  useEffect(() => {
    if (connected) {
      setCoupled(true)
      return
    }
    const timer = setTimeout(() => setCoupled(false), TRAILER_GONE_MS)
    return () => clearTimeout(timer)
  }, [connected])
  return coupled
}

/** At or under this a spring's reading is the plugin's "not read" (-1): no spring stretches a metre. */
const UNREAD = -0.9

/** The springs' heights over their 0 (metres, up positive; `null`: not known). */
function useHeights(): [number | null, number | null, number | null] {
  const suspension = useLive((state) => state.telemetry?.suspension)
  return [0, 1, 2].map((at) => {
    const pressed = suspension?.[at]
    // -1 is "not read" (the plugin's); a spring raised past where it rests reads below 0 too, a little (owner
    // 2026-11-19: "No reading" came and went as the suspension went up and down, though the game was read).
    if (pressed === undefined || pressed <= UNREAD) return null
    if (zero[at] === null) zero[at] = pressed
    // Pressed in less: higher.
    return (zero[at] as number) - pressed
  }) as [number | null, number | null, number | null]
}

/** A number eased toward its target every frame, a spring (critically damped): the drawing moves as air does. */
function useSpring(target: number): number {
  const [value, setValue] = useState(target)
  const state = useRef({ value: target, speed: 0, target })
  state.current.target = target
  useEffect(() => {
    let frame = 0
    let last = performance.now()
    const step = (now: number) => {
      const s = state.current
      const dt = Math.min(0.05, (now - last) / 1000)
      last = now
      const stiffness = 90
      const damping = 2 * Math.sqrt(stiffness)
      s.speed += ((s.target - s.value) * stiffness - s.speed * damping) * dt
      s.value += s.speed * dt
      if (Math.abs(s.target - s.value) < 0.01 && Math.abs(s.speed) < 0.01) {
        s.value = s.target
        s.speed = 0
      }
      setValue(s.value)
      frame = requestAnimationFrame(step)
    }
    frame = requestAnimationFrame(step)
    return () => cancelAnimationFrame(frame)
  }, [])
  return value
}

const px = (metres: number | null) => Math.max(-MOST_PX, Math.min(MOST_PX, (metres ?? 0) * PX_PER_M))

/** Each command held while `held` says, let go as it stops (or the app closes). */
function useHeld(): [string[], (commands: string[]) => void] {
  const [held, setHeld] = useState<string[]>([])
  const current = useRef<string[]>([])
  const hold = (commands: string[]) => {
    for (const command of current.current) {
      if (!commands.includes(command)) postToHost({ type: 'truckCommand', command, down: false })
    }
    for (const command of commands) {
      if (!current.current.includes(command)) postToHost({ type: 'truckCommand', command, down: true })
    }
    current.current = commands
    setHeld(commands)
  }
  useEffect(
    () => () => {
      for (const command of current.current) postToHost({ type: 'truckCommand', command, down: false })
    },
    [],
  )
  return [held, hold]
}

export function SuspensionApp({ words }: { words: SuspensionWords }) {
  const [front, rear, trailer] = useHeights()
  const coupled = useCoupled()
  const known = front !== null || rear !== null
  const [part, setPart] = useState<Part>('all')
  const [held, hold] = useHeld()
  const [preset, setPreset] = useState<{ name: 'high' | 'low' | 'level'; until: number } | null>(null)
  const timers = useRef<ReturnType<typeof setTimeout>[]>([])
  const autoReset = useCharacterSwitches((state) => state.suspensionAutoReset)
  const auto = useCharacterSwitches((state) => state.suspensionAuto)
  const autoDelay = useCharacterSwitches((state) => state.suspensionAutoDelay)
  const autoReturn = useCharacterSwitches((state) => state.suspensionAutoReturn)
  const autoBrake = useCharacterSwitches((state) => state.suspensionAutoBrake)
  const applied = useCharacterSwitches((state) => state.suspensionApplied)
  const lowFront = useCharacterSwitches((state) => state.suspensionLowFront)
  const damping = useCharacterSwitches((state) => state.suspensionDamping)
  // The engine off (owner 2026-11-19: "it doesn't work with the truck off; tell the player"): the air suspension
  // moves only with it running. Unknown (no reading yet): not said.
  const engineOff = useLive((state) => state.telemetry?.engineEnabled === false)
  // A press with it off: the notice shaken, so the player sees why nothing moves.
  const [nudged, setNudged] = useState(0)
  // The trailer gone: back to the truck.
  useEffect(() => {
    if (!coupled && part === 'trailer') setPart('all')
  }, [coupled, part])
  useEffect(() => () => timers.current.forEach(clearTimeout), [])

  const stopPreset = () => {
    timers.current.forEach(clearTimeout)
    timers.current = []
    setPreset(null)
  }
  const nudge = () => setNudged((n) => n + 1)
  const press = (way: 'up' | 'down', down: boolean) => {
    if (down && engineOff) nudge()
    stopPreset()
    hold(down ? COMMANDS[part][way] : [])
  }
  const runPreset = (name: 'high' | 'low') => {
    if (engineOff) nudge()
    stopPreset()
    const commands = [
      ...COMMANDS.all[name === 'high' ? 'up' : 'down'],
      ...(coupled ? COMMANDS.trailer[name === 'high' ? 'up' : 'down'] : []),
    ]
    hold(commands)
    setPreset({ name, until: performance.now() + PRESET_MS })
    timers.current.push(
      setTimeout(() => {
        hold([])
        setPreset(null)
      }, PRESET_MS),
    )
  }
  const level = () => {
    if (engineOff) nudge()
    stopPreset()
    hold([])
    postToHost({ type: 'truckCommand', command: 'suspreset', down: true })
    timers.current.push(setTimeout(() => postToHost({ type: 'truckCommand', command: 'suspreset', down: false }), 200))
    setPreset({ name: 'level', until: performance.now() + SETTLE_MS })
    // Settled: where the springs are now is the new 0.
    timers.current.push(
      setTimeout(() => {
        const suspension = useLive.getState().telemetry?.suspension
        if (suspension) zero = suspension.map((pressed) => (pressed > UNREAD ? pressed : null)) as typeof zero
        setPreset(null)
      }, SETTLE_MS),
    )
  }

  const rising = held.some((command) => command.endsWith('up'))
  const lowering = held.some((command) => command.endsWith('dwn'))
  const moving = (commands: string[]) => commands.some((command) => held.includes(command))
  const air = {
    front: moving(['frontsuspup', 'frontsuspdwn']) ? (rising ? 'up' : 'down') : null,
    rear: moving(['rearsuspup', 'rearsuspdwn']) ? (rising ? 'up' : 'down') : null,
    trailer: moving(['trlrsuspup', 'trlrsuspdwn']) ? (rising ? 'up' : 'down') : null,
  } as const

  return (
    <div className={`susp${engineOff ? ' engine-off' : ''}`}>
      <header className="susp-hero">
        <div className="susp-title">
          <span>{words.title}</span>
          <span className={`susp-status${engineOff ? ' off' : rising ? ' up' : lowering ? ' down' : ''}`}>
            {engineOff
              ? words.engineOffStatus
              : rising
              ? words.rising
              : lowering
                ? words.lowering
                : preset?.name === 'level'
                  ? words.levelling
                  : words.ready}
          </span>
        </div>
        <Rig front={front} rear={rear} trailer={trailer} withTrailer={coupled} air={air} part={part} />
        {!known && <div className="susp-unknown">{words.noReading}</div>}
      </header>

      {engineOff && (
        // Its key: shaken again at each press.
        <div key={nudged} className={`susp-engine-off${nudged > 0 ? ' nudged' : ''}`} role="alert">
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M12 3 22 20H2Zm-1 6v5h2V9Zm0 7v2h2v-2Z" fillRule="evenodd" />
          </svg>
          <div>
            <strong>{words.engineOffTitle}</strong>
            <span>{words.engineOffText}</span>
          </div>
        </div>
      )}

      <div className="susp-gauges">
        <Gauge label={words.front} metres={front} active={part === 'all' || part === 'front'} />
        <Gauge label={words.rear} metres={rear} active={part === 'all' || part === 'rear'} />
        <Gauge label={words.trailer} metres={coupled ? trailer : null} active={part === 'trailer'} off={!coupled} />
      </div>

      <div className="susp-parts" role="radiogroup">
        {(['all', 'front', 'rear', 'trailer'] as const).map((each) => (
          <button
            key={each}
            type="button"
            role="radio"
            aria-checked={part === each}
            disabled={each === 'trailer' && !coupled}
            className={`susp-part${part === each ? ' on' : ''}`}
            onClick={() => setPart(each)}
          >
            <svg viewBox="0 0 32 20" aria-hidden>
              {PART_GLYPHS[each]}
            </svg>
            <span>{words.parts[each]}</span>
          </button>
        ))}
      </div>

      <div className="susp-pad">
        <HoldButton way="up" label={words.up} active={rising} press={(down) => press('up', down)} />
        <HoldButton way="down" label={words.down} active={lowering} press={(down) => press('down', down)} />
      </div>

      <div className="susp-presets">
        <Preset
          label={words.highest}
          glyph={ARROWS_UP}
          running={preset?.name === 'high'}
          run={() => runPreset('high')}
        />
        <Preset label={words.level} glyph={LEVEL} running={preset?.name === 'level'} run={level} />
        <Preset
          label={words.lowest}
          glyph={ARROWS_DOWN}
          running={preset?.name === 'low'}
          run={() => runPreset('low')}
        />
      </div>

      {/* The suspension by itself (owner 2026-11-19): the app lowers it on the motorway, back in town. */}
      <List header={words.auto} footer={words.autoHint}>
        <Row
          icon={<RowIcon tone="#30d158">{ARROWS_DOWN}</RowIcon>}
          label={
            <Segmented
              options={[
                ['on', words.on],
                ['off', words.off],
              ]}
              chosen={auto ? 'on' : 'off'}
              choose={(chosen) => postToHost({ type: 'suspensionAuto', on: chosen === 'on' })}
            />
          }
        />
        {auto && (
          <Row
            label={
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6, width: '100%' }}>
                <span className="ios-row-detail">{words.autoDelay}</span>
                <Segmented
                  options={[
                    ['1', '1s'],
                    ['2', '2s'],
                    ['3', '3s'],
                    ['4', '4s'],
                    ['5', '5s'],
                  ]}
                  chosen={String(autoDelay)}
                  choose={(chosen) => {
                    const secs = Number(chosen)
                    useCharacterSwitches.setState({ suspensionAutoDelay: secs })
                    postToHost({ type: 'suspensionAutoDelay', secs })
                  }}
                />
              </div>
            }
          />
        )}
        {auto && (
          <Row
            label={
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6, width: '100%' }}>
                <span className="ios-row-detail">{words.autoReturn}</span>
                <Segmented
                  options={[
                    ['3', '3s'],
                    ['5', '5s'],
                    ['7', '7s'],
                    ['10', '10s'],
                    ['15', '15s'],
                  ]}
                  chosen={String(autoReturn)}
                  choose={(chosen) => {
                    const secs = Number(chosen)
                    useCharacterSwitches.setState({ suspensionAutoReturn: secs })
                    postToHost({ type: 'suspensionAutoReturn', secs })
                  }}
                />
              </div>
            }
          />
        )}
        {auto && (
          <Row
            label={
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6, width: '100%' }}>
                <span className="ios-row-detail">{words.autoBrake}</span>
                <Segmented
                  options={[
                    ['1', '1s'],
                    ['2', '2s'],
                    ['3', '3s'],
                    ['4', '4s'],
                    ['5', '5s'],
                  ]}
                  chosen={String(autoBrake)}
                  choose={(chosen) => {
                    const secs = Number(chosen)
                    useCharacterSwitches.setState({ suspensionAutoBrake: secs })
                    postToHost({ type: 'suspensionAutoBrake', secs })
                  }}
                />
              </div>
            }
          />
        )}
      </List>
      <List header={words.autoReset} footer={applied === false ? words.autoResetPending : words.autoResetHint}>
        <Row
          icon={<RowIcon tone="#ff9f0a">{LEVEL}</RowIcon>}
          label={
            <Segmented
              options={[
                ['on', words.on],
                ['off', words.off],
              ]}
              chosen={autoReset ? 'on' : 'off'}
              choose={(chosen) => postToHost({ type: 'suspensionAutoReset', on: chosen === 'on' })}
            />
          }
        />
      </List>
      <List header={words.lowFront} footer={words.lowFrontHint}>
        <Row
          label={
            <Segmented
              options={[
                ['normal', words.lowNormal],
                ['low', words.lowLow],
                ['lowest', words.lowLowest],
              ]}
              chosen={lowFront}
              choose={(which) => postToHost({ type: 'suspensionLowFront', which })}
            />
          }
        />
      </List>
      <List header={words.damping} footer={words.dampingHint}>
        <Row
          label={
            <Segmented
              options={[
                ['soft', words.soft],
                ['normal', words.lowNormal],
                ['firm', words.firm],
                ['firmest', words.firmest],
              ]}
              chosen={damping}
              choose={(which) => postToHost({ type: 'suspensionDamping', which })}
            />
          }
        />
      </List>
      <p className="susp-note">{words.limits}</p>
    </div>
  )
}

/** The rig from the side: wheels on the ground, the body on its air springs at the heights given (eased). A modern
 * European tractor drawn to its real proportions (owner 2026-11-18: "a nice truck, like a real one"), 32 px a metre:
 * 1 m wheels, a 3.8 m wheelbase, a cab 2.3 m long and nearly 4 m tall. The cab and everything on the chassis ride on
 * the springs, turned about the front axle as the front and the rear stand; the trailer's kingpin on the fifth wheel,
 * its rear on its own springs. The springs show (as an X-ray) only while they move. */
function Rig({
  front,
  rear,
  trailer,
  withTrailer,
  air,
  part,
}: {
  front: number | null
  rear: number | null
  trailer: number | null
  /** Whether the trailer is drawn (coupled, steadily: `useCoupled`); its height 0 while its springs aren't read. */
  withTrailer: boolean
  air: { front: 'up' | 'down' | null; rear: 'up' | 'down' | null; trailer: 'up' | 'down' | null }
  part: Part
}) {
  const f = useSpring(px(front))
  const r = useSpring(px(rear))
  const t = useSpring(px(withTrailer ? trailer : null))
  const ground = 132
  const wheelY = 116
  const frontAxle = 62
  const rearAxle = 184
  const rail = 98
  // The truck's body: up by the front's height, turned about the front axle by the rear's against it.
  const tilt = (Math.atan2(f - r, rearAxle - frontAxle) * 180) / Math.PI
  const body = `translate(0 ${-f}) rotate(${tilt} ${frontAxle} ${rail})`
  // How high the chassis is over a place along it (the kingpin on the fifth wheel).
  const lift = (x: number) => f + ((r - f) * (x - frontAxle)) / (rearAxle - frontAxle)
  // The trailer: its kingpin (x 180) where the fifth wheel is, its axles (x 492) up by its own springs.
  const kingpinX = 180
  const trailerAxle = 492
  const kingpinLift = lift(kingpinX)
  const trailerTilt = (Math.atan2(kingpinLift - t, trailerAxle - kingpinX) * 180) / Math.PI
  const trailerBody = `translate(0 ${-kingpinLift}) rotate(${trailerTilt} ${kingpinX} 92)`
  const width = withTrailer ? 590 : 230
  const moving = (which: 'up' | 'down' | null) => which !== null
  return (
    <svg className="susp-rig" viewBox={`0 0 ${width} 142`} aria-hidden>
      <defs>
        <linearGradient id="susp-paint" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#7d9cbc" />
          <stop offset="0.18" stopColor="#5a7d9f" />
          <stop offset="0.62" stopColor="#3f6283" />
          <stop offset="1" stopColor="#2a4560" />
        </linearGradient>
        <linearGradient id="susp-paint-side" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#000" stopOpacity="0.28" />
          <stop offset="0.12" stopColor="#000" stopOpacity="0" />
          <stop offset="0.9" stopColor="#000" stopOpacity="0" />
          <stop offset="1" stopColor="#000" stopOpacity="0.22" />
        </linearGradient>
        <linearGradient id="susp-glass" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#9fc3e6" />
          <stop offset="0.45" stopColor="#3d5e80" />
          <stop offset="1" stopColor="#1b2b3c" />
        </linearGradient>
        <linearGradient id="susp-chrome" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#f5f5f7" />
          <stop offset="0.5" stopColor="#a1a1a6" />
          <stop offset="1" stopColor="#e5e5ea" />
        </linearGradient>
        <linearGradient id="susp-plastic" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#3a3a3c" />
          <stop offset="1" stopColor="#1c1c1e" />
        </linearGradient>
        <linearGradient id="susp-box" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#fbfbfd" />
          <stop offset="0.85" stopColor="#d9dbe0" />
          <stop offset="1" stopColor="#b9bcc4" />
        </linearGradient>
        <radialGradient id="susp-tyre" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0.55" stopColor="#3a3a3c" />
          <stop offset="0.8" stopColor="#1c1c1e" />
          <stop offset="1" stopColor="#0b0b0c" />
        </radialGradient>
        <radialGradient id="susp-headlight" cx="0.3" cy="0.5" r="0.7">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="1" stopColor="#c9d6e3" />
        </radialGradient>
      </defs>

      {/* The road: tarmac, the shadow under the rig, the dashes drifting by. */}
      <rect className="susp-tarmac" x="0" y={ground} width={width} height="10" />
      <line className="susp-road" x1="0" y1={ground + 6} x2={width} y2={ground + 6} />
      <ellipse className="susp-shadow" cx={withTrailer ? 300 : 118} cy={ground} rx={withTrailer ? 292 : 112} ry="3.5" />

      {withTrailer && (
        <g className={`susp-trailer${part === 'trailer' ? ' focus' : ''}`}>
          <Wheel x={trailerAxle - 42} y={wheelY} dual />
          <Wheel x={trailerAxle} y={wheelY} dual />
          <Wheel x={trailerAxle + 42} y={wheelY} dual />
          {moving(air.trailer) && (
            <Bellows x={trailerAxle} top={88 - t + 4} bottom={wheelY - 14} air={air.trailer} focus />
          )}
          <g transform={trailerBody}>
            {/* The landing legs, the frame, the side guard. */}
            <rect className="susp-leg" x="206" y="92" width="4" height="24" />
            <rect className="susp-leg-foot" x="202" y="114" width="12" height="3" rx="1" />
            <rect className="susp-frame" x="146" y="88" width="432" height="5" />
            <rect className="susp-guard-rail" x="226" y="98" width="206" height="3" rx="1" />
            <rect className="susp-guard-rail" x="226" y="104" width="206" height="3" rx="1" />
            {/* The box: its panels, its stripe, its rear doors and lights, the under-run bar. */}
            <rect className="susp-box" fill="url(#susp-box)" x="142" y="4" width="440" height="86" rx="3" />
            {Array.from({ length: 9 }, (_, i) => (
              <line key={i} className="susp-panel" x1={142 + 48 * (i + 1)} y1="6" x2={142 + 48 * (i + 1)} y2="88" />
            ))}
            <rect className="susp-stripe" x="146" y="80" width="432" height="4" />
            <rect className="susp-tail" x="577" y="74" width="5" height="9" rx="1" />
            <rect className="susp-underrun" x="548" y="100" width="34" height="5" rx="1" />
          </g>
        </g>
      )}

      {/* The truck's wheels: a single at the front, a pair behind. */}
      <Wheel x={frontAxle} y={wheelY} />
      <Wheel x={rearAxle} y={wheelY} dual />

      {/* The body, on the springs. */}
      <g transform={body}>
        {/* The chassis frame, the fifth wheel, the rear: mudguard and flap, lights. */}
        <rect className="susp-rail" x="40" y={rail} width="176" height="6" rx="1.5" />
        <path className="susp-hitch" d="M166 92 L198 92 L196 98 L168 98 Z" />
        <path
          className="susp-guard"
          fill="url(#susp-plastic)"
          d="M160 104 A26 26 0 0 1 208 104 L208 108 L204 108 A22 22 0 0 0 164 108 L160 108 Z"
        />
        <rect className="susp-flap" x="209" y="104" width="4" height="22" rx="1" />
        <rect className="susp-tail" x="212" y="96" width="5" height="7" rx="1" />
        {/* The side fairing over the tank, between the wheels: the cab's colour, a dark sill. */}
        <path className="susp-fairing" fill="url(#susp-paint)" d="M92 96 L156 96 Q160 96 160 100 L160 118 L92 118 Z" />
        <rect className="susp-sill" fill="url(#susp-plastic)" x="92" y="114" width="68" height="5" />
        <rect className="susp-cap" fill="url(#susp-chrome)" x="132" y="100" width="7" height="4" rx="2" />
        {/* The cab extender behind the cab. */}
        <path className="susp-extender" fill="url(#susp-paint)" d="M93 14 L100 16 L100 92 L93 92 Z" />
        {/* The cab: a slightly raked front, the roof's curve, the arch over the front wheel. */}
        <path
          className="susp-cab"
          fill="url(#susp-paint)"
          d="M13 116 L13 102 L15 52 Q16 44 18 40 L26 14 Q28 8 36 7 L86 4 Q94 4 94 12 L94 116 L86 116 A24 24 0 0 0 38 116 Z"
        />
        <path
          className="susp-cab-shade"
          fill="url(#susp-paint-side)"
          d="M13 116 L13 102 L15 52 Q16 44 18 40 L26 14 Q28 8 36 7 L86 4 Q94 4 94 12 L94 116 L86 116 A24 24 0 0 0 38 116 Z"
        />
        {/* The windscreen seen edge-on, and the side window over the door. */}
        <path className="susp-screen" fill="url(#susp-glass)" d="M18 42 L26 15 Q28 11 33 11 L36 11 L32 44 Z" />
        <path className="susp-glass" fill="url(#susp-glass)" d="M38 12 L74 10 Q78 10 78 14 L78 46 L36 49 Z" />
        <path className="susp-glare" d="M42 16 L60 14 L44 40 Z" />
        {/* The door and its handle; the belt line along the cab. */}
        <path className="susp-line" d="M34 10 L31 104 M80 9 L80 104 M31 104 L80 104" />
        <path className="susp-belt" d="M16 52 L93 50" />
        <rect className="susp-handle" fill="url(#susp-chrome)" x="70" y="58" width="8" height="2.4" rx="1.2" />
        {/* The steps behind the front wheel. */}
        <rect className="susp-step" fill="url(#susp-plastic)" x="84" y="94" width="9" height="3" rx="1" />
        <rect className="susp-step" fill="url(#susp-plastic)" x="85" y="102" width="8" height="3" rx="1" />
        {/* The front: the grille's slats, the headlight, the bumper, a marker light on the side. */}
        <path className="susp-grille" d="M14 60 L18 60 M14 66 L18 66 M14 72 L18 72 M14 78 L18 78 M14 84 L18 84" />
        <rect className="susp-headlight" fill="url(#susp-headlight)" x="12" y="92" width="14" height="6" rx="2" />
        <path className="susp-bumper" fill="url(#susp-plastic)" d="M11 100 L40 100 L40 112 L13 112 Q11 112 11 110 Z" />
        <rect className="susp-marker" x="44" y="99" width="6" height="2.5" rx="1" />
        {/* The mirror on its arm off the front pillar, and the roof's lights. */}
        <path className="susp-mirror-arm" d="M22 26 L8 28 M22 46 L9 44" />
        <rect className="susp-mirror" fill="url(#susp-plastic)" x="4" y="24" width="7" height="24" rx="2.5" />
        <rect className="susp-roof-light" x="40" y="5" width="4" height="2" rx="1" />
        <rect className="susp-roof-light" x="52" y="4.5" width="4" height="2" rx="1" />
        {/* A shine along the cab's shoulder. */}
        <path className="susp-shine" d="M30 9 L88 6" />
      </g>

      {/* The springs, while they move: seen through the body. */}
      {moving(air.front) && <Bellows x={frontAxle} top={rail - f + 2} bottom={wheelY - 14} air={air.front} focus />}
      {moving(air.rear) && <Bellows x={rearAxle} top={rail - r + 2} bottom={wheelY - 14} air={air.rear} focus />}
    </svg>
  )
}

/** An air spring between the axle and the body: its length the height, its air flowing while it's moved. */
function Bellows({
  x,
  top,
  bottom,
  air,
  focus,
}: {
  x: number
  top: number
  bottom: number
  air: 'up' | 'down' | null
  focus: boolean
}) {
  const height = Math.max(4, bottom - top)
  const folds = 4
  const d = Array.from({ length: folds + 1 }, (_, i) => {
    const y = top + (height * i) / folds
    const wide = i % 2 === 0 ? 7 : 9
    return `${i === 0 ? 'M' : 'L'}${x - wide} ${y}`
  }).join(' ')
  const back = Array.from({ length: folds + 1 }, (_, i) => {
    const y = top + (height * (folds - i)) / folds
    const wide = (folds - i) % 2 === 0 ? 7 : 9
    return `L${x + wide} ${y}`
  }).join(' ')
  return (
    <g className={`susp-bellows${focus ? ' focus' : ''}${air ? ` air-${air}` : ''}`}>
      <path d={`${d} ${back} Z`} />
      {air && <line className="susp-flow" x1={x} y1={top + 2} x2={x} y2={bottom - 2} />}
    </g>
  )
}

/** A wheel: its tyre, and a rim with its nuts (a pair behind: a deep dark rim). */
function Wheel({ x, y, dual = false }: { x: number; y: number; dual?: boolean }) {
  return (
    <g className="susp-wheel">
      <circle cx={x} cy={y} r="16" fill="url(#susp-tyre)" />
      <circle className="susp-sidewall" cx={x} cy={y} r="13.4" />
      <circle
        className={`susp-rim${dual ? ' dual' : ''}`}
        cx={x}
        cy={y}
        r="9.5"
        fill={dual ? undefined : 'url(#susp-chrome)'}
      />
      <circle className="susp-hub" cx={x} cy={y} r="3.6" />
      {Array.from({ length: 10 }, (_, i) => (i * 36 * Math.PI) / 180).map((angle) => (
        <circle key={angle} className="susp-nut" cx={x + 6 * Math.cos(angle)} cy={y + 6 * Math.sin(angle)} r="0.85" />
      ))}
    </g>
  )
}

/** A spring's height as a bar either side of 0, and in centimetres. */
function Gauge({
  label,
  metres,
  active,
  off = false,
}: {
  label: string
  metres: number | null
  active: boolean
  off?: boolean
}) {
  const cm = metres === null ? null : metres * 100
  const shown = useSpring(cm ?? 0)
  const fill = Math.max(-1, Math.min(1, shown / GAUGE_CM))
  return (
    <div className={`susp-gauge${active ? ' active' : ''}${off ? ' off' : ''}`}>
      <div className="susp-gauge-bar">
        <span className="susp-gauge-zero" />
        <span
          className={`susp-gauge-fill${fill >= 0 ? ' up' : ' down'}`}
          style={{ height: `${Math.abs(fill) * 50}%`, [fill >= 0 ? 'bottom' : 'top']: '50%' }}
        />
      </div>
      <div className="susp-gauge-value">
        {cm === null ? '—' : `${cm > 0.05 ? '+' : ''}${cm.toFixed(1).replace('.', ',')} cm`}
      </div>
      <div className="susp-gauge-label">{label}</div>
    </div>
  )
}

/** A big round button held as long as it's pressed. */
function HoldButton({
  way,
  label,
  active,
  press,
}: {
  way: 'up' | 'down'
  label: string
  active: boolean
  press: (down: boolean) => void
}) {
  const down = useRef(false)
  const send = (now: boolean) => {
    if (down.current === now) return
    down.current = now
    press(now)
  }
  return (
    <button
      type="button"
      className={`susp-hold ${way}${active ? ' active' : ''}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture?.(event.pointerId)
        send(true)
      }}
      onPointerUp={() => send(false)}
      onPointerCancel={() => send(false)}
      onLostPointerCapture={() => send(false)}
      onContextMenu={(event) => event.preventDefault()}
    >
      <span className="susp-hold-ring" />
      <svg viewBox="0 0 24 24" aria-hidden>
        {way === 'up' ? <path d="M6 15l6-6 6 6" /> : <path d="M6 9l6 6 6-6" />}
      </svg>
      <span className="susp-hold-label">{label}</span>
    </button>
  )
}

/** A preset at a tap: its ring fills while it runs. */
function Preset({
  label,
  glyph,
  running,
  run,
}: {
  label: string
  glyph: ReactNode
  running: boolean
  run: () => void
}) {
  return (
    <button type="button" className={`susp-preset${running ? ' running' : ''}`} onClick={run}>
      <span className="susp-preset-icon">
        <svg viewBox="0 0 24 24" aria-hidden>
          {glyph}
        </svg>
      </span>
      <span>{label}</span>
    </button>
  )
}

/** The parts' pictures: a little truck, its moving part lit. */
const TRUCK_SHAPE = 'M2 13 L2 6 Q2 3 5 3 L11 3 L11 13 Z M11 10 L26 10 L26 13 L11 13 Z'
const PART_GLYPHS: Record<Part, ReactNode> = {
  all: (
    <>
      <path d={TRUCK_SHAPE} className="lit" />
      <circle cx="7" cy="15" r="2.6" />
      <circle cx="22" cy="15" r="2.6" />
    </>
  ),
  front: (
    <>
      <path d={TRUCK_SHAPE} />
      <circle cx="7" cy="15" r="2.6" className="lit" />
      <circle cx="22" cy="15" r="2.6" />
      <path d="M7 1 L7 6" className="arrow" />
    </>
  ),
  rear: (
    <>
      <path d={TRUCK_SHAPE} />
      <circle cx="7" cy="15" r="2.6" />
      <circle cx="22" cy="15" r="2.6" className="lit" />
      <path d="M22 4 L22 9" className="arrow" />
    </>
  ),
  trailer: (
    <>
      <path d="M2 4 L30 4 L30 12 L2 12 Z" className="lit" />
      <circle cx="22" cy="15" r="2.6" />
      <circle cx="28" cy="15" r="2.6" />
    </>
  ),
}

const ARROWS_UP = (
  <path
    d="M7 13l5-5 5 5M7 19l5-5 5 5"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.2"
    strokeLinecap="round"
    strokeLinejoin="round"
  />
)
const ARROWS_DOWN = (
  <path
    d="M7 5l5 5 5-5M7 11l5 5 5-5"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.2"
    strokeLinecap="round"
    strokeLinejoin="round"
  />
)
const LEVEL = (
  <path
    d="M3 12h18M7 8l-4 4 4 4M17 8l4 4-4 4"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  />
)

export interface SuspensionWords {
  title: string
  ready: string
  rising: string
  lowering: string
  levelling: string
  noReading: string
  engineOffStatus: string
  engineOffTitle: string
  engineOffText: string
  front: string
  rear: string
  trailer: string
  parts: Record<Part, string>
  up: string
  down: string
  highest: string
  level: string
  lowest: string
  auto: string
  autoHint: string
  autoDelay: string
  autoReturn: string
  autoBrake: string
  autoReset: string
  autoResetHint: string
  autoResetPending: string
  on: string
  off: string
  limits: string
  lowFront: string
  lowFrontHint: string
  lowNormal: string
  lowLow: string
  lowLowest: string
  damping: string
  dampingHint: string
  soft: string
  firm: string
  firmest: string
}

export const SUSPENSION_WORDS: Record<'en' | 'pt', SuspensionWords> = {
  en: {
    title: 'Air suspension',
    ready: 'Ready',
    rising: 'Raising…',
    lowering: 'Lowering…',
    levelling: 'Levelling…',
    noReading: 'No reading from the game yet: get in the truck (the game restarted after an update).',
    engineOffStatus: 'Engine off',
    engineOffTitle: 'The truck is off',
    engineOffText: 'The air suspension only moves with the engine running. Start it (Commands › Ignition) and try again.',
    front: 'Front',
    rear: 'Rear',
    trailer: 'Trailer',
    parts: { all: 'Truck', front: 'Front', rear: 'Rear', trailer: 'Trailer' },
    up: 'Raise',
    down: 'Lower',
    highest: 'Highest',
    level: 'Level',
    lowest: 'Lowest',
    auto: 'Automatic suspension',
    autoHint:
      'As real trucks do: a while over 80 km/h it lowers a little; under 60 km/h, back to normal. On a rough road it goes up and stays up until the road has been smooth for a while. Braking hard for a while, it rises a little from where it is, and goes back once you let go. Coupling a heavy trailer standing still, it raises the rear back to where it was. Only with the engine running; moving it yourself here pauses it for a minute. Works best with "Back to normal by itself" off.',
    autoDelay: 'Raise delay (rough road)',
    autoReturn: 'Return delay (smooth road)',
    autoBrake: 'Raise delay (braking)',
    autoReset: 'Back to normal by itself',
    autoResetHint:
      'On, the game sets the suspension back to its normal height as you drive off. Off, it stays where you leave it.',
    autoResetPending:
      'Saved: it takes effect once the game is closed and opened again (the game keeps it in its own options).',
    on: 'On',
    off: 'Off',
    limits: 'Raising, the game’s own limits hold: each truck goes as high as its air suspension goes.',
    lowFront: 'Front lower than the game',
    lowFrontHint:
      'Lets the front go further down with Lower (the trucks of the game and its DLCs). It counts from the next time the game starts. Normal leaves the trucks as the game has them.',
    lowNormal: 'Normal',
    lowLow: 'Low',
    lowLowest: 'Lowest',
    damping: 'Dampers',
    dampingHint:
      'How firmly the truck’s and the trailer’s dampers hold the springs: soft rolls and bounces more, firm feels each bump. It counts from the next time the game starts.',
    soft: 'Soft',
    firm: 'Firm',
    firmest: 'Firmest',
  },
  pt: {
    title: 'Suspensão a ar',
    ready: 'Pronta',
    rising: 'Subindo…',
    lowering: 'Descendo…',
    levelling: 'Nivelando…',
    noReading: 'Sem leitura do jogo ainda: entre no caminhão (reinicie o jogo depois de uma atualização).',
    engineOffStatus: 'Desligado',
    engineOffTitle: 'O caminhão está desligado',
    engineOffText: 'A suspensão a ar só sobe e desce com o motor ligado. Ligue o motor (Comandos › Ignição) e tente de novo.',
    front: 'Dianteira',
    rear: 'Traseira',
    trailer: 'Carreta',
    parts: { all: 'Caminhão', front: 'Dianteira', rear: 'Traseira', trailer: 'Carreta' },
    up: 'Subir',
    down: 'Baixar',
    highest: 'Máxima',
    level: 'Nivelar',
    lowest: 'Mínima',
    auto: 'Suspensão automática',
    autoHint:
      'Como nos caminhões de verdade: um tempo acima de 80 km/h ela baixa um pouco; abaixo de 60 km/h, volta ao normal. Em estrada ruim ela sobe e fica em cima até a estrada ficar lisa um bom tempo. Freando forte por um tempo, ela sobe um pouquinho de onde estiver e volta quando você solta o freio. Engatando uma carreta pesada parado, ela sobe a traseira de volta para onde estava. Só com o motor ligado; mexer você mesmo aqui pausa ela por 1 minuto. Funciona melhor com "Voltar ao normal sozinha" desligado.',
    autoDelay: 'Tempo para subir (terreno ruim)',
    autoReturn: 'Tempo para voltar (asfalto liso)',
    autoBrake: 'Tempo para subir (frenagem)',
    autoReset: 'Voltar ao normal sozinha',
    autoResetHint:
      'Ligado, o jogo volta a suspensão para a altura normal quando você anda. Desligado, ela fica onde você deixar.',
    autoResetPending:
      'Salvo: vale depois que o jogo for fechado e aberto de novo (o jogo guarda isso nas opções dele).',
    on: 'Ligado',
    off: 'Desligado',
    limits: 'Para subir, os limites do jogo continuam: cada caminhão sobe até onde a suspensão a ar dele vai.',
    lowFront: 'Frente mais baixa que o jogo',
    lowFrontHint:
      'Deixa a frente descer mais com o Baixar (caminhões do jogo e das DLCs). Vale na próxima vez que o jogo abrir. Normal deixa os caminhões como o jogo tem.',
    lowNormal: 'Normal',
    lowLow: 'Baixa',
    lowLowest: 'Máxima',
    damping: 'Amortecedor',
    dampingHint:
      'O quanto os amortecedores do caminhão e da carreta seguram as molas: macio balança e pula mais, firme sente cada buraco. Vale na próxima vez que o jogo abrir.',
    soft: 'Macio',
    firm: 'Firme',
    firmest: 'Muito firme',
  },
}
