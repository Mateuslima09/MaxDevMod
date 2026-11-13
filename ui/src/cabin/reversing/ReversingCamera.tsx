import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { cabClock } from '../../shared/host'
import { coverLiveMap } from '../maps/liveMap'
import { NO_MAP, useMapData } from '../maps/mapData'
import { isNight } from '../state/appearance'
import { useLive } from '../state/store'
import { useText } from '../state/text'
import { Beeper } from './beeper'
import { useReversingSettings } from './settings'
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
  useEffect(() => {
    coverLiveMap(shown)
    if (!shown) beeper.set(null)
  }, [shown])
  return shown ? <CameraScreen /> : null
}

/** One for the page: its audio made once, the first time something's near. */
const beeper = new Beeper()

/** How often the latch looks at the telemetry when nothing changes (its delay out of reverse), ms. */
const LATCH_EVERY_MS = 200

function useReversingShown(): boolean {
  const enabled = useReversingSettings((state) => state.camera)
  const [shown, setShown] = useState(false)
  const latch = useRef(new ReverseLatch())
  useEffect(() => {
    if (!enabled) {
      setShown(false)
      return
    }
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
  return shown
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
  const [muted, setMuted] = useState(false)
  const [frame, setFrame] = useState<Frame | null>(null)
  const sound = useRef({ beeps, muted })
  useLayoutEffect(() => {
    sound.current = { beeps, muted }
  })

  useEffect(() => {
    cameraView().guides = guides
  }, [guides])

  useEffect(() => {
    if (!host.current) return
    const view = cameraView()
    let last = ''
    view.show(host.current, data ?? NO_MAP, night ? 'dark' : 'light', (next) => {
      const live = useLive.getState()
      const silent = !sound.current.beeps || sound.current.muted || !live.telemetry || live.telemetry.paused || !live.gameShown
      beeper.set(silent ? null : beepingFor(nearestOf(next.readings)))
      const shown = { ...next, readings: next.readings.map(shownReading), articulation: Math.round(next.articulation * 50) / 50 }
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
    <div className={`reversing ${night ? 'night' : 'day'}`} role="img" aria-label={text.reversing.title}>
      <div ref={host} className="reversing-host" />
      <div className="reversing-badge">
        <CameraGlyph />
        <span>{text.reversing.title}</span>
        <span className="reversing-where">{frame?.trailer ? text.reversing.onTrailer : text.reversing.onTruck}</span>
      </div>
      <div className="reversing-caution">{text.reversing.checkSurroundings}</div>
      <aside className="reversing-panel">
        <button
          type="button"
          className={`reversing-mute${muted || !beeps ? ' off' : ''}`}
          aria-label={muted ? text.reversing.unmute : text.reversing.mute}
          onClick={() => setMuted(!muted)}
        >
          <SpeakerGlyph off={muted || !beeps} />
        </button>
        <SensorDiagram frame={frame} />
        <div className={`reversing-distance${nearest === null ? ' clear' : ` ${zoneOf(nearest)}`}`}>
          {nearest === null
            ? '—'
            : nearest <= STEADY_AT
              ? text.reversing.stop
              : text.reversing.distance(nearest.toFixed(1).replace('.', decimal))}
        </div>
      </aside>
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
      <rect className="body" x={cx - halfWidth} y={rearY - bodyLength} width={halfWidth * 2} height={bodyLength} rx={6} />
      {trailer && (
        // The truck ahead, turned about the kingpin as the trailer is to it.
        <g transform={`rotate(${(articulation * 180) / Math.PI} ${cx} ${rearY - bodyLength + 10})`}>
          <rect className="body truck" x={cx - halfWidth + 2} y={rearY - bodyLength - 46} width={halfWidth * 2 - 4} height={62} rx={7} />
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
