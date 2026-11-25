import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { cabClock, type CameraNudge, hasVideoUnder, postToHost } from '../../shared/host'
import { coverLiveMap } from '../maps/liveMap'
import { NO_MAP, useMapData } from '../maps/mapData'
import { isNight } from '../state/appearance'
import { useLive } from '../state/store'
import { useText } from '../state/text'
import { beeper } from './beeper'
import { drawOurGuides } from './ourCamera'
import { BEEP_VOLUMES, useReversingSettings } from './settings'
import { beepingFor, nearestOf, SENSORS, STEADY_AT, zoneOf } from './sensor'
import { ReverseLatch } from './shown'
import { cameraView, type Frame } from './view'
import './reversing.css'

/**
 * The reversing camera, over the whole screen while the truck is in reverse (shown.ts): the camera's picture
 * (view.ts) with its guide lines, and beside it the parking sensors as seen from above, the rig with its trailer
 * turned as it is and each sensor's band lit by how near it sees something, the nearest distance, and the button
 * that mutes their beeps (until the next time in reverse). The beeps (beeper.ts) sound whatever the camera shows.
 */
export function ReversingCamera() {
  const shown = useReversingShown()
  const leaving = useLeaving(shown)
  useEffect(() => {
    coverLiveMap(shown || leaving)
    if (!shown) beeper.set(null)
  }, [shown, leaving])
  if (shown) return <CameraScreen />
  // Out of reverse: the screen covered a moment while the plugin lets its camera go, then the screen as it was.
  return leaving ? (
    <div className="reversing">
      <Cover gone={false} />
    </div>
  ) : null
}

/**
 * The switch into reverse and back covered (owner 2026-11-19: "going in, the screen shows the game's map, then the
 * plain mirror, then the camera; and the same glitch coming out"): going in, a card over the whole screen until the
 * plugin's camera has settled (`cameraReady`; without the game's video under the page, the map's camera: its
 * first frame), at most [`COVER_MOST_MS`]; coming out, the card a moment ([`LEAVING_MS`]) while the camera goes.
 */
const COVER_MOST_MS = 3500
const LEAVING_MS = 450
/** How long the card takes to fade (ms, as reversing.css has it). */
const COVER_FADE_MS = 200

/** Whether it's just come out of reverse (for [`LEAVING_MS`]). */
function useLeaving(shown: boolean): boolean {
  const [leaving, setLeaving] = useState(false)
  const was = useRef(shown)
  useEffect(() => {
    const left = was.current && !shown
    was.current = shown
    if (!left) {
      setLeaving(false)
      return
    }
    setLeaving(true)
    const timer = setTimeout(() => setLeaving(false), LEAVING_MS)
    return () => clearTimeout(timer)
  }, [shown])
  return leaving
}

/** The card over the reversing screen: the camera's name, and a bar running while it comes up. */
function Cover({ gone }: { gone: boolean }) {
  const text = useText()
  return (
    <div className={`reversing-cover${gone ? ' gone' : ''}`} aria-hidden={gone}>
      <div className="reversing-cover-card">
        <CameraGlyph />
        <span>{text.reversing.title}</span>
        <div className="reversing-cover-bar" />
      </div>
    </div>
  )
}

/** How often the latch looks at the telemetry when nothing changes (its delay out of reverse), ms. */
const LATCH_EVERY_MS = 200

function useReversingShown(): boolean {
  const enabled = useReversingSettings((state) => state.camera)
  const [shown, setShown] = useState(false)
  const latch = useRef(new ReverseLatch())
  useEffect(() => {
    // Turned off in Settings: not shown (below), nothing watched.
    if (!enabled) return
    const check = () => setShown(latch.current.update(useLive.getState().telemetry, performance.now()))
    check()
    const stop = useLive.subscribe((live, before) => {
      if (live.telemetry !== before.telemetry) check()
    })
    const timer = setInterval(check, LATCH_EVERY_MS)
    return () => {
      stop()
      clearInterval(timer)
    }
  }, [enabled])
  return enabled && shown
}

/** The readings as shown: to 5 cm, so the screen changes only as they do. */
const shownReading = (reading: number | null) => (reading === null ? null : Math.round(reading * 20) / 20)

function CameraScreen() {
  const text = useText()
  const host = useRef<HTMLDivElement>(null)
  const data = useMapData()
  const night = useLive((state) => (state.telemetry ? isNight(cabClock(state.telemetry, state.clockShift)) : true))
  const guides = useReversingSettings((state) => state.guides)
  const beeps = useReversingSettings((state) => state.beeps)
  const volume = useReversingSettings((state) => BEEP_VOLUMES[state.beepVolume] ?? 1)
  useEffect(() => beeper.setVolume(volume), [volume])
  const [muted, setMuted] = useState(false)
  // Moving the camera by hand (owner 2026-11-20): its buttons in the sensors' place meanwhile.
  const [adjusting, setAdjusting] = useState(false)
  const [frame, setFrame] = useState<Frame | null>(null)
  // The game's own mirrors in the picture's place, while it draws them (the camera in the cab): MirrorHole.
  const mirror = useLive((state) => state.telemetry?.mirror === true) && hasVideoUnder()
  // The card over it going in: until the plugin's camera has settled (the game's video under the page: it draws
  // it), else the map's camera's first frame; at most COVER_MOST_MS. Then faded and gone.
  const cameraReady = useLive((state) => state.telemetry?.cameraReady === true)
  const [covered, setCovered] = useState(true)
  const [coverShown, setCoverShown] = useState(true)
  const settled = hasVideoUnder() ? cameraReady : frame !== null
  useEffect(() => {
    if (!covered) return
    const timer = setTimeout(() => setCovered(false), settled ? 0 : COVER_MOST_MS)
    return () => clearTimeout(timer)
  }, [covered, settled])
  useEffect(() => {
    if (covered) return
    const timer = setTimeout(() => setCoverShown(false), COVER_FADE_MS)
    return () => clearTimeout(timer)
  }, [covered])
  // Out of reverse, the plugin lets the mirrors go at once while the screen waits a moment for reverse again
  // (shown.ts): the camera from the map came back meanwhile (owner 2026-11-19: "going ahead, the map shows a
  // moment"). Once the plugin's camera has been seen, its mirror gone is covered, not the map.
  const [mirrorSeen, setMirrorSeen] = useState(false)
  useEffect(() => {
    if (mirror) setMirrorSeen(true)
  }, [mirror])
  const lost = mirrorSeen && !mirror
  const sound = useRef({ beeps, muted })
  useLayoutEffect(() => {
    sound.current = { beeps, muted }
  })

  useEffect(() => {
    cameraView().guides = guides
  }, [guides])

  // The beeps only with the camera in the cab (owner 2026-11-14: outside it they only get in the way): quiet at once
  // as it leaves (the picture holds still out of view, so its next reading may not come).
  const inCab = useLive((state) => state.screenSeen)
  useEffect(() => {
    if (!inCab) beeper.set(null)
  }, [inCab])

  useEffect(() => {
    if (!host.current) return
    const view = cameraView()
    let last = ''
    view.show(host.current, data ?? NO_MAP, night ? 'dark' : 'light', (next) => {
      const live = useLive.getState()
      const silent =
        !sound.current.beeps ||
        sound.current.muted ||
        !live.telemetry ||
        live.telemetry.paused ||
        !live.gameShown ||
        !live.screenSeen
      beeper.set(silent ? null : beepingFor(nearestOf(next.readings)))
      const shown = {
        ...next,
        readings: next.readings.map(shownReading),
        articulation: Math.round(next.articulation * 50) / 50,
      }
      const key = JSON.stringify(shown)
      if (key !== last) {
        last = key
        setFrame(shown)
      }
    })
    return () => view.hide()
  }, [data, night])

  const nearest = frame ? nearestOf(frame.readings) : null
  const decimal = text.decimal
  return (
    <div
      className={`reversing ${night ? 'night' : 'day'}${mirror ? ' mirror' : ''}`}
      role="img"
      aria-label={text.reversing.title}
    >
      <div ref={host} className="reversing-host" />
      {mirror && <MirrorHole guides={guides} />}
      <div className="reversing-badge">
        <CameraGlyph />
        <span>{text.reversing.title}</span>
        {/* Which it's on once the first frame says (not "Truck" for a moment on a trailer). */}
        {frame && (
          <span className="reversing-where">{frame.trailer ? text.reversing.onTrailer : text.reversing.onTruck}</span>
        )}
      </div>
      <div className="reversing-caution">{text.reversing.checkSurroundings}</div>
      {(coverShown || lost) && <Cover gone={!covered && !lost} />}
      <aside className="reversing-panel">
        <div className="reversing-panel-buttons">
          {/* Only where the plugin's camera is (the game's video under the page): the map's camera isn't moved. */}
          {mirror && (
            <button
              type="button"
              className={`reversing-mute reversing-adjust${adjusting ? ' on' : ''}`}
              aria-label={text.reversing.adjust}
              onClick={() => setAdjusting(!adjusting)}
            >
              <AdjustGlyph />
            </button>
          )}
          <button
            type="button"
            className={`reversing-mute${muted || !beeps ? ' off' : ''}`}
            aria-label={muted ? text.reversing.unmute : text.reversing.mute}
            onClick={() => setMuted(!muted)}
          >
            <SpeakerGlyph off={muted || !beeps} />
          </button>
        </div>
        {adjusting && mirror ? (
          <AdjustPad done={() => setAdjusting(false)} />
        ) : (
          <>
            <SensorDiagram frame={frame} />
            <div className={`reversing-distance${nearest === null ? ' clear' : ` ${zoneOf(nearest)}`}`}>
              {nearest === null
                ? '—'
                : nearest <= STEADY_AT
                  ? text.reversing.stop
                  : text.reversing.distance(nearest.toFixed(1).replace('.', decimal))}
            </div>
          </>
        )}
      </aside>
    </div>
  )
}

/** How often the mirrors' place is told again (the host may have started anew meanwhile), ms. */
const MIRROR_TOLD_EVERY_MS = 1000

/**
 * Where our camera shows (crates/plugin mirror.rs, rear_camera.rs): the page left see-through over the whole
 * picture, the rest of the CarPlay area hidden under it, and the host told where, in the frame's pixels; the
 * plugin draws the camera there under the page (its badge, the sensors' panel over it, owner 2026-11-14, and our
 * guide lines: ourCamera.ts).
 */
function MirrorHole({ guides }: { guides: boolean }) {
  const hole = useRef<HTMLDivElement>(null)
  const lines = useRef<HTMLCanvasElement>(null)
  useEffect(() => {
    const canvas = lines.current
    if (!canvas) return
    let frame = 0
    const draw = () => {
      drawOurGuides(canvas, guides ? useLive.getState().telemetry : null)
      frame = requestAnimationFrame(draw)
    }
    draw()
    return () => cancelAnimationFrame(frame)
  }, [guides])
  useEffect(() => {
    const element = hole.current
    if (!element) return
    const page = document.documentElement
    page.classList.add('mirror-camera')
    const tell = () => {
      const box = element.getBoundingClientRect()
      const scale = window.devicePixelRatio
      postToHost({
        type: 'mirrorCamera',
        x: box.left * scale,
        y: box.top * scale,
        width: box.width * scale,
        height: box.height * scale,
      })
    }
    tell()
    const resized = new ResizeObserver(tell)
    resized.observe(element)
    const timer = setInterval(tell, MIRROR_TOLD_EVERY_MS)
    return () => {
      resized.disconnect()
      clearInterval(timer)
      page.classList.remove('mirror-camera')
      postToHost({ type: 'mirrorCamera', x: 0, y: 0, width: 0, height: 0 })
    }
  }, [])
  return (
    <div ref={hole} className="reversing-mirror">
      <canvas ref={lines} className="reversing-guides" />
    </div>
  )
}

/** The bands round the rear, nearest first: up to each one's distance (m), the sensors' zones. */
const BANDS = [0.8, 1.6, 3]
/** Each sensor's sector round the rear, degrees either side of straight back (left of the picture is left). */
const SECTORS: [number, number][] = [
  [-70, -36],
  [-34, -1],
  [1, 34],
  [36, 70],
]

/**
 * The rig from above, its front up: the trailer (or the truck's own rear) with its rear at the sensors' bands,
 * the truck ahead of it turned as it is to it, and each sensor's sector lit in the band of its reading.
 */
function SensorDiagram({ frame }: { frame: Frame | null }) {
  const width = 200
  const height = 300
  const cx = width / 2
  const rearY = 150
  const halfWidth = 26
  const scale = 26
  const radius = (metres: number) => 14 + metres * scale
  const sector = (from: number, to: number, inner: number, outer: number) => {
    // Angles from straight down (back), positive to the right of the rig (the screen's right, as the picture).
    const point = (angle: number, r: number) => {
      const a = (angle * Math.PI) / 180
      return `${cx + Math.sin(a) * r},${rearY + Math.cos(a) * r}`
    }
    return `M${point(from, inner)} A${inner},${inner} 0 0 0 ${point(to, inner)} L${point(to, outer)} A${outer},${outer} 0 0 1 ${point(from, outer)} Z`
  }
  const articulation = frame?.articulation ?? 0
  const trailer = frame?.trailer ?? false
  const bodyLength = trailer ? 120 : 70
  return (
    <svg className="reversing-diagram" viewBox={`0 0 ${width} ${height}`} aria-hidden>
      {SENSORS.map((_, index) => {
        const reading = frame?.readings[index] ?? null
        const lit = reading === null ? -1 : BANDS.findIndex((band) => reading <= band)
        const [from, to] = SECTORS[index]
        return BANDS.map((band, at) => (
          <path
            key={`${index}-${at}`}
            d={sector(from, to, radius(at === 0 ? 0 : BANDS[at - 1]) + 1.5, radius(band) - 1.5)}
            className={`band ${at === lit ? `lit ${['red', 'yellow', 'green'][at]}` : at < lit || lit === -1 ? '' : 'beyond'}`}
          />
        ))
      })}
      {/* The rear's body (the trailer, or the truck alone), up from its rear. */}
      <rect
        className="body"
        x={cx - halfWidth}
        y={rearY - bodyLength}
        width={halfWidth * 2}
        height={bodyLength}
        rx={6}
      />
      {trailer && (
        // The truck ahead, turned about the kingpin as the trailer is to it.
        <g transform={`rotate(${(articulation * 180) / Math.PI} ${cx} ${rearY - bodyLength + 10})`}>
          <rect
            className="body truck"
            x={cx - halfWidth + 2}
            y={rearY - bodyLength - 46}
            width={halfWidth * 2 - 4}
            height={62}
            rx={7}
          />
        </g>
      )}
    </svg>
  )
}

function CameraGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor" aria-hidden>
      <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5h8A2.5 2.5 0 0 1 16 7.5v1.3l3.6-2.2A.9.9 0 0 1 21 7.4v9.2a.9.9 0 0 1-1.4.8L16 15.2v1.3a2.5 2.5 0 0 1-2.5 2.5h-8A2.5 2.5 0 0 1 3 16.5Z" />
    </svg>
  )
}

/** A signed distance as the adjust pad shows it: "+0,50 m". */
function nudgeText(metres: number, decimal: string): string {
  const sign = metres > 0 ? '+' : metres < 0 ? '−' : ''
  return `${sign}${Math.abs(metres).toFixed(2).replace('.', decimal)} m`
}

/**
 * The camera moved by hand (owner 2026-11-20: "can I adjust the reversing camera myself?"): further back or forward,
 * higher or lower, 5 cm a press (owner 2026-11-20: a quarter metre was too much), or back to where it goes by itself
 * (turning it, too: the phone's Câmera de Ré app); the plugin keeps it for the truck or
 * trailer it's on (rear_camera.rs `nudged`) and says where it is now (the telemetry's `cameraBack`, `cameraUp`).
 */
function AdjustPad({ done }: { done: () => void }) {
  const text = useText()
  const back = useLive((state) => state.telemetry?.cameraBack ?? 0)
  const up = useLive((state) => state.telemetry?.cameraUp ?? 0)
  const side = useLive((state) => state.telemetry?.cameraSide ?? 0)
  const roll = useLive((state) => state.telemetry?.cameraRoll ?? 0)
  const press = (action: CameraNudge) => postToHost({ type: 'cameraNudge', action })
  const words = text.reversing
  const rollText = `${roll > 0 ? '+' : roll < 0 ? '−' : ''}${Math.abs(Math.round(roll * 10) / 10)}°`
  return (
    <div className="reversing-adjust-pad">
      <strong>{words.adjust}</strong>
      <div className="reversing-adjust-row">
        <button type="button" onClick={() => press('back')}>
          {words.adjustBack}
        </button>
        <button type="button" onClick={() => press('backFar')}>
          +50 cm
        </button>
      </div>
      <div className="reversing-adjust-row">
        <button type="button" onClick={() => press('forward')}>
          {words.adjustForward}
        </button>
        <button type="button" onClick={() => press('forwardFar')}>
          −50 cm
        </button>
      </div>
      <span className="reversing-adjust-value">{nudgeText(back, text.decimal)}</span>
      <div className="reversing-adjust-row">
        <button type="button" onClick={() => press('up')}>
          {words.adjustUp}
        </button>
        <button type="button" onClick={() => press('down')}>
          {words.adjustDown}
        </button>
      </div>
      <span className="reversing-adjust-value">{nudgeText(up, text.decimal)}</span>
      <div className="reversing-adjust-row">
        <button type="button" onClick={() => press('left')}>
          {words.adjustLeft}
        </button>
        <button type="button" onClick={() => press('right')}>
          {words.adjustRight}
        </button>
      </div>
      <span className="reversing-adjust-value">{nudgeText(side, text.decimal)}</span>
      <div className="reversing-adjust-row">
        <button type="button" onClick={() => press('rollLeft')}>
          {words.adjustRollLeft}
        </button>
        <button type="button" onClick={() => press('rollRight')}>
          {words.adjustRollRight}
        </button>
      </div>
      <span className="reversing-adjust-value">{rollText}</span>
      <div className="reversing-adjust-row">
        <button type="button" className="quiet" onClick={() => press('reset')}>
          {words.adjustReset}
        </button>
        <button type="button" className="quiet" onClick={done}>
          {words.adjustDone}
        </button>
      </div>
      <small>{words.adjustNote}</small>
    </div>
  )
}

function AdjustGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      <path
        d="M12 3v18M3 12h18M12 3l-2.5 2.5M12 3l2.5 2.5M12 21l-2.5-2.5M12 21l2.5-2.5M3 12l2.5-2.5M3 12l2.5 2.5M21 12l-2.5-2.5M21 12l-2.5 2.5"
        strokeLinecap="round"
      />
    </svg>
  )
}

function SpeakerGlyph({ off }: { off: boolean }) {
  return (
    <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      <path d="M4 9.5h3.2L12 5.5v13l-4.8-4H4Z" fill="currentColor" strokeLinejoin="round" />
      {off ? (
        <path d="m15.5 9.5 5 5m0-5-5 5" strokeLinecap="round" />
      ) : (
        <path d="M15.5 9a4.2 4.2 0 0 1 0 6m2.6-8.6a8 8 0 0 1 0 11.2" strokeLinecap="round" />
      )}
    </svg>
  )
}
