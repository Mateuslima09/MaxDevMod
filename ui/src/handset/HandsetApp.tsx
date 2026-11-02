import { type ReactNode, useEffect, useState } from 'react'
import { formatGameTime, formatGear, speedKmh } from '../shared/format'
import { cabClock } from '../shared/host'
import { MapView } from '../cabin/dashboard/MapView'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import { PhoneVolume } from '../phone/PhoneVolume'
import '../cabin/dashboard/dashboard.css'

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
  const { appearance } = useLook()
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
              {app === 'maps' && <MapView interactive />}
              {app === 'music' && <MusicApp words={words} />}
              {app === 'freight' && <FreightApp words={words} />}
              {app === 'truck' && <TruckApp words={words} />}
              {app === 'settings' && <SettingsApp words={words} />}
            </div>
          </section>
        )}
        <button type="button" className="handset-home" aria-label={words.home} onClick={home} />
      </div>
    </main>
  )
}

type AppId = 'maps' | 'music' | 'freight' | 'truck' | 'settings'

const APPS: AppId[] = ['maps', 'music', 'freight', 'truck', 'settings']

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

/** What plays, larger, with its volume. */
function MusicApp({ words }: { words: Words }) {
  return (
    <>
      <div className="handset-music handset-music-big">
        <NowPlayingWidget />
      </div>
      <div className="handset-card">
        <div className="handset-kicker">{words.volume}</div>
        <PhoneVolume label={words.volume} />
      </div>
      <p className="handset-hint">{words.musicHint}</p>
    </>
  )
}

/** The job, as the game gives it: the cargo and its weight, from where to where, how far yet, what it pays. */
function FreightApp({ words }: { words: Words }) {
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
        <Fact name={words.left} value={left > 0 ? `${Math.round(left / 1000).toLocaleString()} km` : '—'} />
        <Fact name={words.arrives} value={left > 0 ? duration(time / 60) : '—'} />
        <Fact name={words.planned} value={`${job.plannedDistanceKm.toLocaleString()} km`} />
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
function TruckApp({ words }: { words: Words }) {
  const truck = useLive((state) => state.truck)
  const t = useLive((state) => state.telemetry)
  if (!t) return <Empty>{words.noGame}</Empty>
  return (
    <>
      <article className="handset-card">
        <div className="handset-kicker">{truck?.brand ?? ''}</div>
        <div className="handset-cargo">{truck?.name ?? ''}</div>
      </article>
      <div className="handset-gauge">
        <span className="handset-gauge-value">{speedKmh(t.speed)}</span>
        <span className="handset-gauge-unit">km/h</span>
      </div>
      <dl className="handset-facts">
        <Fact name={words.gear} value={formatGear(t.displayedGear)} />
        <Fact name="RPM" value={Math.round(t.engineRpm).toLocaleString()} />
        <Fact name={words.fuel} value={`${Math.round(t.fuel)} L`} />
        <Fact name={words.range} value={`${Math.round(t.fuelRange)} km`} />
        <Fact
          name={words.cruise}
          value={t.cruiseControlSpeed > 0 ? `${speedKmh(t.cruiseControlSpeed)} km/h` : words.off}
        />
        <Fact name={words.engine} value={t.engineEnabled ? words.on : words.off} />
        <Fact name={words.parkingBrake} value={t.parkingBrake ? words.on : words.off} />
        <Fact name={words.lights} value={t.highBeam ? words.high : t.lowBeam ? words.on : words.off} />
      </dl>
    </>
  )
}

/** The volume of what plays, and how the phone is used. */
function SettingsApp({ words }: { words: Words }) {
  return (
    <>
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
    apps: { maps: 'Maps', music: 'Music', freight: 'Job', truck: 'Truck', settings: 'Settings' },
    days: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
    noJob: 'No job right now',
    noGame: 'The game isn’t running',
    delivering: 'Delivering',
    pickingUp: 'Picking up',
    weight: 'Weight',
    pays: 'Pays',
    left: 'Left',
    arrives: 'Arrives in',
    planned: 'Planned',
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
    musicHint: 'Choose what plays on the truck’s screen: Spotify, YouTube Music or Apple Music.',
    howTo: 'How to use',
    howToText:
      'Alt+4 takes the phone out and puts it away. Hold Alt to use it with the mouse. The key can be changed in WestPlay’s Controls.',
  },
  pt: {
    home: 'Início',
    apps: { maps: 'Mapas', music: 'Música', freight: 'Frete', truck: 'Caminhão', settings: 'Ajustes' },
    days: ['Segunda-feira', 'Terça-feira', 'Quarta-feira', 'Quinta-feira', 'Sexta-feira', 'Sábado', 'Domingo'],
    noJob: 'Nenhum frete agora',
    noGame: 'O jogo não está aberto',
    delivering: 'Entregando',
    pickingUp: 'Indo buscar',
    weight: 'Peso',
    pays: 'Paga',
    left: 'Falta',
    arrives: 'Chega em',
    planned: 'Distância',
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
    musicHint: 'Escolha o que tocar na tela do caminhão: Spotify, YouTube Music ou Apple Music.',
    howTo: 'Como usar',
    howToText:
      'Alt+4 tira e guarda o celular. Segure o Alt para mexer nele com o mouse. A tecla pode ser trocada em Controles, no WestPlay.',
  },
}

type Words = (typeof WORDS)['en']
