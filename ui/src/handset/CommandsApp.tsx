import { type ReactNode, useEffect, useRef } from 'react'
import { postToHost } from '../shared/host'
import type { CommandWords } from './commandWords'

/**
 * The truck's commands (owner 2026-11-02: "open the window, lower the truck, the suspension"): each button is one
 * of the game's own controls (`westplay_shared::TRUCK_COMMANDS`), held while it's pressed, as the plugin gives it
 * to the game (crates/plugin commands.rs). The windows and the suspension move as long as they're held, as their
 * keys do; the rest toggle at a tap.
 */
export function CommandsApp({ words }: { words: CommandWords }) {
  return (
    <>
      {/* The key, as it turns (owner 2026-11-11): on (the electrics), the start (held until it runs), off. */}
      <Group title={words.ignition} tone="red">
        <Hold command="ignitionon" label={words.ignitionOn} glyph={KEY} />
        <Hold command="ignitionstrt" label={words.ignitionStart} glyph={START} />
        {/* The engine alone (owner 2026-11-18: E again cuts the key, the electrics and the screen with it): the key
            off, then back on. */}
        <Steps steps={ENGINE_ONLY_OFF} label={words.engineOnlyOff} glyph={ENGINE} />
        <Hold command="ignitionoff" label={words.ignitionOff} glyph={OFF} />
      </Group>
      <Group title={words.windows} tone="blue">
        <Hold command="lwinopen" label={words.leftDown} glyph={DOWN} />
        <Hold command="lwinclose" label={words.leftUp} glyph={UP} />
        <Hold command="rwinopen" label={words.rightDown} glyph={DOWN} />
        <Hold command="rwinclose" label={words.rightUp} glyph={UP} />
      </Group>
      {/* The suspension has its own app now (owner 2026-11-19: "take the suspension out of the commands"). */}
      <Group title={words.truck} tone="orange">
        <Hold command="beacon" label={words.beacon} glyph={BEACON} />
        <Hold command="flasher4way" label={words.hazard} glyph={HAZARD} />
        <Hold command="liftaxle" label={words.liftAxle} glyph={AXLE} />
        <Hold command="cabinlight" label={words.cabinLight} glyph={LIGHT} />
        <Hold command="horn" label={words.horn} glyph={HORN} />
        <Hold command="airhorn" label={words.airHorn} glyph={HORN} />
        <Hold command="wipers" label={words.wipers} glyph={WIPERS} />
        <Hold command="parkingbrake" label={words.parkingBrake} glyph={BRAKE} />
        <Hold command="attach" label={words.attach} glyph={HOOK} wide />
      </Group>
    </>
  )
}

/** The key turned off, then back on: the engine stops, the electrics (and the screen) stay. Each command held this
 * long (ms), then the next after a moment (the game takes a press a frame or two later). */
const ENGINE_ONLY_OFF: [string, number][] = [
  ['ignitionoff', 200],
  ['ignitionon', 200],
]
const STEP_GAP_MS = 500

/** Commands one after another at a tap (`ENGINE_ONLY_OFF`), each pressed and let go. */
function Steps({ steps, label, glyph }: { steps: [string, number][]; label: string; glyph: ReactNode }) {
  const timers = useRef<ReturnType<typeof setTimeout>[]>([])
  useEffect(() => () => timers.current.forEach(clearTimeout), [])
  const run = () => {
    timers.current.forEach(clearTimeout)
    timers.current = []
    let at = 0
    for (const [command, hold] of steps) {
      timers.current.push(setTimeout(() => postToHost({ type: 'truckCommand', command, down: true }), at))
      timers.current.push(setTimeout(() => postToHost({ type: 'truckCommand', command, down: false }), at + hold))
      at += hold + STEP_GAP_MS
    }
  }
  return (
    <button type="button" className="handset-command" onClick={run} onContextMenu={(event) => event.preventDefault()}>
      <span className="handset-command-icon">
        <svg viewBox="0 0 24 24" aria-hidden>
          {glyph}
        </svg>
      </span>
      <span className="handset-command-label">{label}</span>
    </button>
  )
}

/** A group of commands, its buttons in its colour (`tone`). */
function Group({ title, tone, children }: { title: string; tone: string; children: ReactNode }) {
  return (
    <section className={`handset-commands tone-${tone}`}>
      <div className="handset-kicker">{title}</div>
      <div className="handset-command-grid">{children}</div>
    </section>
  )
}

/** A command, held from the press until it's let go (or the pointer leaves it). */
export function Hold({
  command,
  label,
  glyph,
  wide = false,
}: {
  command: string
  label: string
  glyph: ReactNode
  wide?: boolean
}) {
  const held = useRef(false)
  const send = (down: boolean) => {
    if (held.current === down) return
    held.current = down
    postToHost({ type: 'truckCommand', command, down })
  }
  // Let go if the app closes while it's held.
  useEffect(
    () => () => {
      if (held.current) postToHost({ type: 'truckCommand', command, down: false })
    },
    [command],
  )
  return (
    <button
      type="button"
      className={`handset-command${wide ? ' wide' : ''}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture?.(event.pointerId)
        send(true)
      }}
      onPointerUp={() => send(false)}
      onPointerCancel={() => send(false)}
      onLostPointerCapture={() => send(false)}
      onContextMenu={(event) => event.preventDefault()}
    >
      <span className="handset-command-icon">
        <svg viewBox="0 0 24 24" aria-hidden>
          {glyph}
        </svg>
      </span>
      <span className="handset-command-label">{label}</span>
    </button>
  )
}

const KEY = (
  <path
    d="M8 15a4 4 0 1 1 3.5-6H21v3h-2v2h-3v-2h-4.5A4 4 0 0 1 8 15Zm0-2.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z"
    fillRule="evenodd"
  />
)
const START = (
  <>
    <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="2" />
    <path d="M10 8v8l6-4Z" />
  </>
)
const OFF = (
  <path
    d="M12 3v8M7 6.3a7.5 7.5 0 1 0 10 0"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.2"
    strokeLinecap="round"
  />
)
const ENGINE = <path d="M4 9h3l2-2h5v2h3l2 2h2v6h-2l-2 2H8l-2-2H4Z" />
const UP = <path d="M12 5v14M6 11l6-6 6 6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
const DOWN = (
  <path d="M12 19V5M6 13l6 6 6-6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
)
const BEACON = <path d="M7 18v-6a5 5 0 0 1 10 0v6ZM5 18h14v3H5ZM12 2v3M4 6l2 2M20 6l-2 2" />
const HAZARD = <path d="M12 3 22 20H2Zm0 5-5.5 9.5h11Z" fillRule="evenodd" />
const AXLE = (
  <>
    <circle cx="6" cy="16" r="3" />
    <circle cx="18" cy="16" r="3" />
    <path d="M6 16h12M12 4v8M9 7l3-3 3 3" fill="none" stroke="currentColor" strokeWidth="2" />
  </>
)
const LIGHT = <path d="M9 18h6v2H9Zm3-16a7 7 0 0 0-4 12.7V16h8v-1.3A7 7 0 0 0 12 2Z" />
const HORN = <path d="M3 9h4l8-5v16l-8-5H3Zm15 0a4 4 0 0 1 0 6" />
const WIPERS = (
  <path
    d="M12 20 5 8M12 20a10 10 0 0 0-9-10M12 20a10 10 0 0 1 9-10"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
  />
)
const BRAKE = (
  <>
    <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" strokeWidth="2" />
    <path d="M10 8h3a2.5 2.5 0 0 1 0 5h-3Zm0 0v8" fill="none" stroke="currentColor" strokeWidth="2" />
  </>
)
const HOOK = (
  <path d="M12 3v9a4 4 0 1 1-4 4" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
)
