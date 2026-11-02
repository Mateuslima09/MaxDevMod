import { type PointerEvent, type ReactNode, Suspense, useRef, useState } from 'react'
import { formatGameTime } from '../shared/format'
import { cabClock } from '../shared/host'
import { MapView } from './lazy'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { nearestCity } from '../cabin/dashboard/destination'
import { weatherView } from '../cabin/dashboard/weather'
import { WeatherGlyph } from '../cabin/dashboard/WeatherGlyph'
import { EMPTY, useDestinationView } from '../cabin/dashboard/widgetState'
import { useMapData } from '../cabin/maps/mapData'
import { useLive } from '../cabin/state/store'
import { useText } from '../cabin/state/text'
import { type AppId, AppIcon } from './icons'
import { type Prefs, speed, speedUnit } from './prefs'
import { Scaled } from './Scaled'

/** How much smaller the minimap draws the cab's map. */
const MINIMAP_SCALE = 0.45
/** How far a drag goes before it's a swipe to the other page, not a tap (px). */
const SWIPE = 50
/** How far a pointer moves before it's a drag, not a tap (px): a finger's tap wanders a little. */
const DRAG = 14

/** The apps in the dock, on every page; and each page's own. */
const DOCK: AppId[] = ['maps', 'music', 'youtubeMusic', 'spotify']
const PAGE_ONE: AppId[] = ['freight', 'truck', 'commands', 'settings']
const PAGE_TWO: AppId[] = ['nupay', 'services']

export interface HomeWords {
  apps: Record<AppId, string>
  days: string[]
  noGame: string
  openMap: string
}

/**
 * The home screen as the iPhone's (owner 2026-11-03: "everything as on the iPhone", and "a second screen for
 * more apps"): its pages side by side, swiped from one to the other (a finger, or the mouse dragged with the
 * screen's key held), their dots, and the dock under them. The first page has the widgets (the time, the
 * weather or the truck, what plays, where the truck is headed) and the apps; the second, the map.
 */
export function Home({
  words,
  open,
  units,
  remote,
}: {
  words: HomeWords
  open: (app: AppId) => void
  units: Prefs['units']
  remote: boolean
}) {
  const [page, setPage] = useState(0)
  const swipe = useSwipe(page, setPage, 2)
  return (
    <section className="ios-home">
      <div className="ios-pages" {...swipe.handlers}>
        <div className="ios-track" style={{ transform: `translateX(calc(${-page * 50}% + ${swipe.offset}px))` }}>
          <div className="ios-page">
            <div className="ios-widget-row">
              <ClockWidget days={words.days} />
              <WeatherOrTruckWidget units={units} noGame={words.noGame} />
            </div>
            <div className="ios-widget ios-widget-music">
              <NowPlayingWidget />
            </div>
            <ArrivalWidget />
            <Icons apps={PAGE_ONE} words={words} open={open} />
          </div>
          <div className="ios-page">
            {/* The map following the truck, its route on it (owner 2026-11-02: "a tiny minimap, the route"); drawn
                only while its page shows (or is swiped to): a map costs the GPU, which the game needs. On the
                player's own phone, a card that opens Maps: the map drawn as the page was swiped held a phone for
                seconds (its report: 2.7 s, 2 frames a second). */}
            {remote ? (
              <button type="button" className="ios-widget ios-widget-map ios-map-card" onClick={() => open('maps')}>
                <AppIcon id="maps" />
                <span className="ios-arrival-title">{words.apps.maps}</span>
                <span className="ios-widget-kicker">{words.openMap}</span>
              </button>
            ) : (
              <Scaled scale={MINIMAP_SCALE} className="ios-widget ios-widget-map">
                {(page === 1 || swipe.offset !== 0) && (
                  <Suspense fallback={null}>
                    <MapView />
                  </Suspense>
                )}
              </Scaled>
            )}
            <Icons apps={PAGE_TWO} words={words} open={open} />
          </div>
        </div>
      </div>
      <div className="ios-dots" role="tablist">
        {[0, 1].map((dot) => (
          <button
            key={dot}
            type="button"
            role="tab"
            aria-selected={dot === page}
            aria-label={String(dot + 1)}
            className={dot === page ? 'on' : undefined}
            onClick={() => setPage(dot)}
          />
        ))}
      </div>
      <nav className="ios-dock">
        {DOCK.map((id) => (
          <button key={id} type="button" className="ios-app" onClick={() => open(id)} aria-label={words.apps[id]}>
            <AppIcon id={id} />
          </button>
        ))}
      </nav>
    </section>
  )
}

function Icons({ apps, words, open }: { apps: AppId[]; words: HomeWords; open: (app: AppId) => void }) {
  return (
    <nav className="ios-grid">
      {apps.map((id) => (
        <button key={id} type="button" className="ios-app" onClick={() => open(id)}>
          <AppIcon id={id} />
          <span className="ios-app-label">{words.apps[id]}</span>
        </button>
      ))}
    </nav>
  )
}

/**
 * Swiping between pages: the page follows the pointer as it drags sideways, and goes to the next or the last
 * once let go far enough. A drag isn't a tap: the app under it doesn't open.
 */
function useSwipe(page: number, setPage: (page: number) => void, pages: number) {
  const start = useRef<{ x: number; y: number } | null>(null)
  const dragged = useRef(false)
  const [offset, setOffset] = useState(0)
  const end = () => {
    start.current = null
    if (offset <= -SWIPE) setPage(Math.min(pages - 1, page + 1))
    else if (offset >= SWIPE) setPage(Math.max(0, page - 1))
    setOffset(0)
  }
  return {
    offset,
    handlers: {
      onPointerDown: (event: PointerEvent) => {
        start.current = { x: event.clientX, y: event.clientY }
        dragged.current = false
      },
      onPointerMove: (event: PointerEvent) => {
        if (!start.current) return
        const dx = event.clientX - start.current.x
        const dy = event.clientY - start.current.y
        if (!dragged.current && Math.abs(dx) > DRAG && Math.abs(dx) > Math.abs(dy)) {
          dragged.current = true
          event.currentTarget.setPointerCapture?.(event.pointerId)
        }
        // Past the first page or the last, it gives a little only.
        if (dragged.current) setOffset((page === 0 && dx > 0) || (page === pages - 1 && dx < 0) ? dx / 4 : dx)
      },
      onPointerUp: end,
      onPointerCancel: end,
      onClickCapture: (event: { preventDefault(): void; stopPropagation(): void }) => {
        if (!dragged.current) return
        event.preventDefault()
        event.stopPropagation()
        dragged.current = false
      },
    },
  }
}

/** The time in the game and its day, as iOS's small clock widget. */
function ClockWidget({ days }: { days: string[] }) {
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))
  return (
    <article className="ios-widget ios-widget-small ios-widget-clock">
      <div className="ios-widget-kicker">{time === null ? '' : days[Math.floor(time / 1440) % 7]}</div>
      <div className="ios-widget-big">{time === null ? '--:--' : formatGameTime(time)}</div>
    </article>
  )
}

/** The weather where the truck is (as the Dashboard's widget); where the game's isn't known, the truck's speed
 * and fuel instead. */
function WeatherOrTruckWidget({ units, noGame }: { units: Prefs['units']; noGame: string }) {
  const weather = useLive((state) => state.weather)
  // What it shows, as it shows it: drawn again only as that changes, not at each of the game's twenty a second.
  const fuel = useLive((state) => (state.telemetry ? Math.round(state.telemetry.fuel) : null))
  const shownSpeed = useLive((state) => (state.telemetry ? speed(state.telemetry.speed, units) : null))
  if (weather) return <WeatherWidget weather={weather} />
  return (
    <Small className="ios-widget-truck" kicker={fuel === null ? noGame : `${fuel} L`}>
      <div className="ios-widget-big">{shownSpeed ?? '–'}</div>
      <div className="ios-widget-line">{speedUnit(units)}</div>
    </Small>
  )
}

/** The weather and the nearest city: the map's names (the whole map's data, heavy on a phone) only for it. */
function WeatherWidget({ weather }: { weather: NonNullable<ReturnType<typeof useLive.getState>['weather']> }) {
  const text = useText()
  const labels = useMapData()?.labels
  const gameTime = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : 0))
  const place = useLive((state) => {
    const t = state.telemetry
    return t && labels ? nearestCity([t.x, t.z], labels) : null
  })
  const view = weatherView(weather, gameTime, place, text)
  return (
    <article className="ios-widget ios-widget-small ios-widget-weather">
      <div className="ios-widget-kicker">{view.place}</div>
      <WeatherGlyph symbol={view.symbol} size={46} />
      <div className="ios-widget-line">{view.condition}</div>
    </article>
  )
}

function Small({ className, kicker, children }: { className: string; kicker: string; children: ReactNode }) {
  return (
    <article className={`ios-widget ios-widget-small ${className}`}>
      <div className="ios-widget-kicker">{kicker}</div>
      {children}
    </article>
  )
}

/** Where the truck is headed, as the Dashboard's widget (destination.ts): the pickup or the delivery, by when
 * and how far; or where the game's route ends. */
function ArrivalWidget() {
  const view = useDestinationView()
  if (view === EMPTY) return null
  return (
    <article className="ios-widget ios-widget-arrival">
      <span className="ios-arrival-flag" aria-hidden>
        <svg viewBox="0 0 24 24">
          <path d="M6 21V4h11l-2 4 2 4H8v9Z" />
        </svg>
      </span>
      <div className="ios-arrival-text">
        <div className="ios-widget-kicker">{view.kicker}</div>
        <div className="ios-arrival-title">{view.title}</div>
        <div className={`ios-arrival-detail${view.late ? ' late' : ''}`}>
          {[view.subtitle, view.detail].filter(Boolean).join(' · ')}
        </div>
      </div>
    </article>
  )
}
