import { type CSSProperties, type ReactNode, useEffect, useState } from 'react'
import { formatGameTime, formatGear } from '../shared/format'
import { cabClock } from '../shared/host'
import { MapView } from '../cabin/dashboard/MapView'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { nearestCity } from '../cabin/dashboard/destination'
import { weatherView } from '../cabin/dashboard/weather'
import { WeatherGlyph } from '../cabin/dashboard/WeatherGlyph'
import { SpotifyApp } from '../cabin/apps/spotify/SpotifyApp'
import { YouTubeMusicApp } from '../cabin/apps/youtubeMusic/YouTubeMusicApp'
import { useDestinationView, EMPTY } from '../cabin/dashboard/widgetState'
import { Keyboard } from '../cabin/keyboard/Keyboard'
import { useMapData } from '../cabin/maps/mapData'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { languageOf } from '../cabin/state/strings'
import { useText } from '../cabin/state/text'
import { followTimeZone } from '../cabin/state/timeZone'
import { PhoneVolume } from '../phone/PhoneVolume'
import { useFullScreen } from './fullScreen'
import { distance, type Prefs, setPrefs, speed, speedUnit, usePrefs } from './prefs'
import { Scaled } from './Scaled'
import '../cabin/dashboard/dashboard.css'
import '../cabin/apps/apps.css'

/** How much smaller the cab's own screens are drawn in the phone: the map's, and the music apps'. */
const MAP_SCALE = 0.55
const MUSIC_SCALE = 0.5

/**
 * The phone in the game (owner 2026-11-01: "a phone that shows up in the game at a key... as if it were a real
 * phone"), drawn by the plugin over the game's picture while it's out (plugin handset.rs) and used with the
 * mouse while the screen's key is held; also on the player's own phone, from its own QR code (handset/install.ts).
 * A home screen and its apps: Maps, Music, the job, the truck and Settings. From the same host messages the cab
 * has; nothing of the cab's own doing runs here (the cab does it, once).
 */
export function HandsetApp({ remote }: { remote: boolean }) {
  useEffect(() => connectHost(), [])
  useEffect(() => followTimeZone(), [])
  const language = languageOf(useLive((state) => state.gameLanguage))
  useEffect(() => {
    document.documentElement.lang = language
  }, [language])
  useLookChanges()
  const prefs = usePrefs()
  // Light or dark as chosen in its Settings, else as the cab is.
  const cab = useLook().appearance
  const appearance = prefs.look === 'auto' ? cab : prefs.look
  useFullScreen(remote)
  const words = WORDS[language === 'pt' ? 'pt' : 'en']
  const [app, setApp] = useState<AppId | null>(null)
  const home = () => setApp(null)

  return (
    <main className={`handset shell appearance-${appearance}${remote ? ' handset-remote' : ''}`}>
      <div className="handset-screen">
        <StatusBar />
        {app === null ? (
          <Home words={words} open={setApp} />
        ) : (
          <section className="handset-app" key={app}>
            <header className="handset-app-bar">
              <button type="button" className="handset-back" onClick={home}>
                ‹ {words.home}
              </button>
              <h1>{words.apps[app]}</h1>
            </header>
            <div className={`handset-app-body handset-app-${app}`}>
              {app === 'maps' && (
                <CabScreen scale={MAP_SCALE}>
                  <section className="app-panel maps-app">
                    <MapView interactive />
                  </section>
                </CabScreen>
              )}
              {app === 'youtubeMusic' && (
                <CabScreen scale={MUSIC_SCALE}>
                  <YouTubeMusicApp />
                </CabScreen>
              )}
              {app === 'spotify' && (
                <CabScreen scale={MUSIC_SCALE}>
                  <SpotifyApp />
                </CabScreen>
              )}
              {app === 'music' && <MusicApp words={words} open={setApp} />}
              {app === 'freight' && <FreightApp words={words} units={prefs.units} />}
              {app === 'truck' && <TruckApp words={words} units={prefs.units} />}
              {app === 'settings' && <SettingsApp words={words} prefs={prefs} />}
            </div>
          </section>
        )}
        <button type="button" className="handset-home" aria-label={words.home} onClick={home} />
      </div>
    </main>
  )
}

type AppId = 'maps' | 'music' | 'youtubeMusic' | 'spotify' | 'freight' | 'truck' | 'settings'

const APPS: AppId[] = ['maps', 'music', 'youtubeMusic', 'spotify', 'freight', 'truck', 'settings']

/** One of the cab's own screens, smaller (Scaled.tsx), in an app window as the cab has it, with the cab's
 * keyboard for its search fields (the map's, the music apps'). */
function CabScreen({ scale, children }: { scale: number; children: ReactNode }) {
  return (
    <Scaled scale={scale} className="handset-cab">
      <div className="screen app-window" style={{ '--content-left': '16px', '--edge': '16px' } as CSSProperties}>
        {children}
        <Keyboard />
      </div>
    </Scaled>
  )
}

/** The time in the game, the island, and the signal and battery. */
function StatusBar() {
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))
  return (
    <header className="handset-status">
      <span className="handset-time">{time === null ? '--:--' : formatGameTime(time)}</span>
      <span className="handset-island" />
      <span className="handset-signal" aria-hidden>
        <svg viewBox="0 0 34 12" width="34" height="12">
          <rect x="0" y="8" width="3" height="4" rx="1" />
          <rect x="5" y="5" width="3" height="7" rx="1" />
          <rect x="10" y="2" width="3" height="10" rx="1" />
          <rect x="20" y="1" width="12" height="10" rx="3" fill="none" stroke="currentColor" strokeWidth="1.4" />
          <rect x="22" y="3" width="8" height="6" rx="1.5" />
        </svg>
      </span>
    </header>
  )
}

/** The home screen: the clock, what plays, and the apps. */
function Home({ words, open }: { words: Words; open: (app: AppId) => void }) {
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))
  const day = time === null ? null : words.days[Math.floor(time / 1440) % 7]
  return (
    <section className="handset-homescreen">
      <div className="handset-clock">
        <div className="handset-clock-time">{time === null ? '--:--' : formatGameTime(time)}</div>
        {day && <div className="handset-clock-day">{day}</div>}
      </div>
      <div className="handset-music">
        <NowPlayingWidget />
      </div>
      <ArrivalCard />
      <WeatherCard />
      <nav className="handset-grid">
        {APPS.map((id) => (
          <button key={id} type="button" className="handset-icon" onClick={() => open(id)}>
            <span className={`handset-glyph handset-glyph-${id}`}>{GLYPHS[id]}</span>
            <span className="handset-label">{words.apps[id]}</span>
          </button>
        ))}
      </nav>
    </section>
  )
}

/** Where the truck is headed, as the Dashboard's widget (destination.ts): the pickup or the delivery, by when
 * and how far; or where the game's route ends (owner 2026-11-02: "the arrival widget, as CarPlay's"). */
function ArrivalCard() {
  const view = useDestinationView()
  if (view === EMPTY) return null
  return (
    <article className="handset-card handset-arrival">
      <span className="handset-arrival-flag" aria-hidden>
        <svg viewBox="0 0 24 24">
          <path d="M6 21V4h11l-2 4 2 4H8v9Z" />
        </svg>
      </span>
      <div className="handset-arrival-text">
        <div className="handset-kicker">{view.kicker}</div>
        <div className="handset-place-name">{view.title}</div>
        {view.subtitle && <div className="handset-place-city">{view.subtitle}</div>}
        <div className={`handset-arrival-detail${view.late ? ' late' : ''}`}>{view.detail}</div>
      </div>
    </article>
  )
}

/** The weather where the truck is, as the Dashboard's widget (none while the game's isn't known). */
function WeatherCard() {
  const text = useText()
  const labels = useMapData()?.labels
  const weather = useLive((state) => state.weather)
  const gameTime = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : 0))
  const place = useLive((state) => {
    const t = state.telemetry
    return t && labels ? nearestCity([t.x, t.z], labels) : null
  })
  const view = weather ? weatherView(weather, gameTime, place, text) : null
  if (!view) return null
  return (
    <article className="handset-card handset-weather">
      <div>
        {view.place && <div className="handset-place-name">{view.place}</div>}
        <div className="handset-place-city">{view.condition}</div>
      </div>
      <WeatherGlyph symbol={view.symbol} size={44} />
    </article>
  )
}

/** What plays, larger, with its volume, and the music apps to choose it in. */
function MusicApp({ words, open }: { words: Words; open: (app: AppId) => void }) {
  return (
    <>
      <div className="handset-music handset-music-big">
        <NowPlayingWidget />
      </div>
      <div className="handset-card">
        <div className="handset-kicker">{words.volume}</div>
        <PhoneVolume label={words.volume} />
      </div>
      <div className="handset-sources">
        {(['youtubeMusic', 'spotify'] as const).map((id) => (
          <button key={id} type="button" className="handset-source" onClick={() => open(id)}>
            <span className={`handset-glyph handset-glyph-small handset-glyph-${id}`}>{GLYPHS[id]}</span>
            <span>{words.apps[id]}</span>
            <span className="handset-chevron">›</span>
          </button>
        ))}
      </div>
    </>
  )
}

/** The job, as the game gives it: the cargo and its weight, from where to where, how far yet, what it pays. */
function FreightApp({ words, units }: { words: Words; units: Prefs['units'] }) {
  const job = useLive((state) => state.job)
  const left = useLive((state) => state.telemetry?.navigationDistance ?? 0)
  const time = useLive((state) => state.telemetry?.navigationTime ?? 0)
  const now = useLive((state) => state.telemetry?.gameTime ?? null)
  if (!job) return <Empty>{words.noJob}</Empty>
  const due = now === null ? null : job.deliveryTime - now
  return (
    <>
      <article className="handset-card">
        <div className="handset-kicker">{job.cargoLoaded ? words.delivering : words.pickingUp}</div>
        <div className="handset-cargo">{job.cargo}</div>
      </article>
      <dl className="handset-facts">
        <Fact
          name={words.weight}
          value={`${(job.cargoMass / 1000).toLocaleString(undefined, { maximumFractionDigits: 1 })} t`}
        />
        <Fact name={words.pays} value={`$ ${job.income.toLocaleString()}`} />
        <Fact name={words.left} value={left > 0 ? distance(left, units) : words.noRoute} />
        <Fact name={words.arrives} value={left > 0 ? duration(time / 60) : '—'} />
        <Fact name={words.planned} value={distance(job.plannedDistanceKm * 1000, units)} />
        <Fact
          name={words.deadline}
          value={due === null ? '—' : due < 0 ? words.late : duration(due)}
          late={due !== null && due < 0}
        />
      </dl>
      <article className="handset-card handset-route">
        <Place
          kicker={words.from}
          name={job.special ? job.sourceCity : job.sourceCompany}
          city={job.special ? null : job.sourceCity}
        />
        <div className="handset-route-line" />
        <Place
          kicker={words.to}
          name={job.special ? job.destinationCity : job.destinationCompany}
          city={job.special ? null : job.destinationCity}
        />
      </article>
    </>
  )
}

/** The truck as the game says it is now: speed, gear, engine, fuel and its range. */
function TruckApp({ words, units }: { words: Words; units: Prefs['units'] }) {
  const truck = useLive((state) => state.truck)
  const t = useLive((state) => state.telemetry)
  if (!t) return <Empty>{words.noGame}</Empty>
  const fuel = units === 'mi' ? `${Math.round(t.fuel / LITRES_PER_GALLON)} gal` : `${Math.round(t.fuel)} L`
  return (
    <>
      <article className="handset-card">
        <div className="handset-kicker">{truck?.brand ?? ''}</div>
        <div className="handset-cargo">{truck?.name ?? ''}</div>
      </article>
      <div className="handset-gauge">
        <span className="handset-gauge-value">{speed(t.speed, units)}</span>
        <span className="handset-gauge-unit">{speedUnit(units)}</span>
      </div>
      <dl className="handset-facts">
        <Fact name={words.gear} value={formatGear(t.displayedGear)} />
        <Fact name="RPM" value={Math.round(t.engineRpm).toLocaleString()} />
        <Fact name={words.fuel} value={fuel} />
        <Fact name={words.range} value={distance(t.fuelRange * 1000, units)} />
        <Fact
          name={words.cruise}
          value={t.cruiseControlSpeed > 0 ? `${speed(t.cruiseControlSpeed, units)} ${speedUnit(units)}` : words.off}
        />
        <Fact name={words.engine} value={t.engineEnabled ? words.on : words.off} />
        <Fact name={words.parkingBrake} value={t.parkingBrake ? words.on : words.off} />
        <Fact name={words.lights} value={t.highBeam ? words.high : t.lowBeam ? words.on : words.off} />
      </dl>
    </>
  )
}

const LITRES_PER_GALLON = 3.785411784

/** The phone's look and units (prefs.ts), the volume of what plays, and how the phone is used. */
function SettingsApp({ words, prefs }: { words: Words; prefs: Prefs }) {
  return (
    <>
      <div className="handset-card">
        <div className="handset-kicker">{words.look}</div>
        <Choice
          options={[
            ['auto', words.lookAuto],
            ['light', words.lookLight],
            ['dark', words.lookDark],
          ]}
          chosen={prefs.look}
          choose={(look) => setPrefs({ look })}
        />
      </div>
      <div className="handset-card">
        <div className="handset-kicker">{words.units}</div>
        <Choice
          options={[
            ['mi', words.miles],
            ['km', words.kilometres],
          ]}
          chosen={prefs.units}
          choose={(units) => setPrefs({ units })}
        />
      </div>
      <div className="handset-card">
        <div className="handset-kicker">{words.volume}</div>
        <PhoneVolume label={words.volume} />
      </div>
      <div className="handset-card">
        <div className="handset-kicker">{words.howTo}</div>
        <p className="handset-text">{words.howToText}</p>
      </div>
      <p className="handset-hint">WestPlay ATS</p>
    </>
  )
}

/** A choice of a few, as iOS's segmented control. */
function Choice<T extends string>({
  options,
  chosen,
  choose,
}: {
  options: [T, string][]
  chosen: T
  choose: (value: T) => void
}) {
  return (
    <div className="handset-choice" role="radiogroup">
      {options.map(([value, label]) => (
        <button
          key={value}
          type="button"
          role="radio"
          aria-checked={value === chosen}
          className={value === chosen ? 'on' : undefined}
          onClick={() => choose(value)}
        >
          {label}
        </button>
      ))}
    </div>
  )
}

function Fact({ name, value, late = false }: { name: string; value: string; late?: boolean }) {
  return (
    <div className="handset-fact">
      <dt>{name}</dt>
      <dd className={late ? 'late' : undefined}>{value}</dd>
    </div>
  )
}

function Place({ kicker, name, city }: { kicker: string; name: string; city: string | null }) {
  return (
    <div className="handset-place">
      <div className="handset-kicker">{kicker}</div>
      <div className="handset-place-name">{name}</div>
      {city && <div className="handset-place-city">{city}</div>}
    </div>
  )
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="handset-empty">{children}</p>
}

/** Game minutes as "45 min" or "3 h 05". */
function duration(minutes: number): string {
  const whole = Math.max(0, Math.round(minutes))
  return whole < 60 ? `${whole} min` : `${Math.floor(whole / 60)} h ${String(whole % 60).padStart(2, '0')}`
}

const GLYPHS: Record<AppId, ReactNode> = {
  youtubeMusic: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <path d="M10 8.5v7l6-3.5Z" />
    </svg>
  ),
  spotify: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path
        d="M5 9c4.5-1.4 9.5-1 13.5 1.2M6 12.6c3.8-1.1 7.8-.7 11 1M7 16c3-.8 6-.5 8.6.8"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  ),
  maps: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M12 2.5 19 20l-7-4-7 4Z" />
    </svg>
  ),
  music: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M9 17.5V5l11-2v12.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <circle cx="6.5" cy="17.5" r="2.8" />
      <circle cx="17.5" cy="15.5" r="2.8" />
    </svg>
  ),
  freight: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M3 7.5 12 3l9 4.5v9L12 21l-9-4.5Z" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M3 7.5 12 12l9-4.5M12 12v9" fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  ),
  truck: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path d="M2 6h11v10H2ZM13 9h4.5l3.5 4v3h-8Z" />
      <circle cx="6" cy="17.5" r="2.2" fill="#fff" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="17" cy="17.5" r="2.2" fill="#fff" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  ),
  settings: (
    <svg viewBox="0 0 24 24" aria-hidden>
      <path
        d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Zm8.5 3.5 2-1.6-2-3.5-2.4.9a7.5 7.5 0 0 0-2-1.2L15.7 4h-4l-.4 2.6a7.5 7.5 0 0 0-2 1.2l-2.4-.9-2 3.5 2 1.6a7.5 7.5 0 0 0 0 2.3l-2 1.6 2 3.5 2.4-.9a7.5 7.5 0 0 0 2 1.2l.4 2.5h4l.4-2.5a7.5 7.5 0 0 0 2-1.2l2.4.9 2-3.5-2-1.6a7.5 7.5 0 0 0 0-2.3Z"
        fillRule="evenodd"
      />
    </svg>
  ),
}

const WORDS = {
  en: {
    home: 'Home',
    apps: {
      maps: 'Maps',
      music: 'Music',
      youtubeMusic: 'YT Music',
      spotify: 'Spotify',
      freight: 'Job',
      truck: 'Truck',
      settings: 'Settings',
    },
    days: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
    noJob: 'No job right now',
    noGame: 'The game isn’t running',
    delivering: 'Delivering',
    pickingUp: 'Picking up',
    weight: 'Weight',
    pays: 'Pays',
    left: 'Left on the route',
    arrives: 'Arrives in',
    planned: 'Whole job',
    deadline: 'Deadline',
    late: 'Late',
    from: 'From',
    to: 'To',
    gear: 'Gear',
    fuel: 'Fuel',
    range: 'Range',
    cruise: 'Cruise',
    engine: 'Engine',
    parkingBrake: 'Parking brake',
    lights: 'Lights',
    on: 'On',
    off: 'Off',
    high: 'High',
    volume: 'Volume',
    noRoute: 'No route',
    look: 'Look',
    lookAuto: 'Automatic',
    lookLight: 'Light',
    lookDark: 'Dark',
    units: 'Units',
    miles: 'Miles (mph)',
    kilometres: 'Kilometres (km/h)',
    howTo: 'How to use',
    howToText:
      'Alt+4 takes the phone out and puts it away. Hold Alt to use it with the mouse. The key can be changed in WestPlay’s Controls.',
  },
  pt: {
    home: 'Início',
    apps: {
      maps: 'Mapas',
      music: 'Música',
      youtubeMusic: 'YT Music',
      spotify: 'Spotify',
      freight: 'Frete',
      truck: 'Caminhão',
      settings: 'Ajustes',
    },
    days: ['Segunda-feira', 'Terça-feira', 'Quarta-feira', 'Quinta-feira', 'Sexta-feira', 'Sábado', 'Domingo'],
    noJob: 'Nenhum frete agora',
    noGame: 'O jogo não está aberto',
    delivering: 'Entregando',
    pickingUp: 'Indo buscar',
    weight: 'Peso',
    pays: 'Paga',
    left: 'Falta na rota',
    arrives: 'Chega em',
    planned: 'Frete inteiro',
    deadline: 'Prazo',
    late: 'Atrasado',
    from: 'De',
    to: 'Para',
    gear: 'Marcha',
    fuel: 'Diesel',
    range: 'Autonomia',
    cruise: 'Piloto',
    engine: 'Motor',
    parkingBrake: 'Freio de mão',
    lights: 'Faróis',
    on: 'Ligado',
    off: 'Desligado',
    high: 'Alto',
    volume: 'Volume',
    noRoute: 'Sem rota',
    look: 'Aparência',
    lookAuto: 'Automático',
    lookLight: 'Claro',
    lookDark: 'Escuro',
    units: 'Unidades',
    miles: 'Milhas (mph)',
    kilometres: 'Quilômetros (km/h)',
    howTo: 'Como usar',
    howToText:
      'Alt+4 tira e guarda o celular. Segure o Alt para mexer nele com o mouse. A tecla pode ser trocada em Controles, no WestPlay.',
  },
}

type Words = (typeof WORDS)['en']
