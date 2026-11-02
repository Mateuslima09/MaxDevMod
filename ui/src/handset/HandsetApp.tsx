import { type CSSProperties, type ReactNode, useEffect, useState } from 'react'
import { formatGameTime, formatGear } from '../shared/format'
import { cabClock } from '../shared/host'
import { MapView } from '../cabin/dashboard/MapView'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { SpotifyApp } from '../cabin/apps/spotify/SpotifyApp'
import { YouTubeMusicApp } from '../cabin/apps/youtubeMusic/YouTubeMusicApp'
import { Keyboard } from '../cabin/keyboard/Keyboard'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import { PhoneVolume } from '../phone/PhoneVolume'
import { useNowPlaying } from '../cabin/dashboard/useNowPlaying'
import { CommandsApp } from './CommandsApp'
import { COMMAND_WORDS } from './commandWords'
import { goFullScreen } from './fullScreen'
import { Home } from './Home'
import { type AppId, AppIcon, RowIcon } from './icons'
import { List, Row, Segmented } from './ios'
import { distance, type Prefs, setPrefs, speed, speedUnit, usePrefs, WALLPAPERS } from './prefs'
import { NupayApp } from './NupayApp'
import { ServicesApp } from './ServicesApp'
import { SERVICES_WORDS } from './servicesWords'
import { Scaled } from './Scaled'
import '../cabin/dashboard/dashboard.css'
import '../cabin/apps/apps.css'

/** How much smaller the cab's own screens are drawn in the phone: the map's, and the music apps'. */
const MAP_SCALE = 0.55
const MUSIC_SCALE = 0.5
/** The apps that show one of the cab's screens: a compact bar, the screen the rest. */
const CAB_APPS: AppId[] = ['maps', 'youtubeMusic', 'spotify']

/**
 * The phone in the game (owner 2026-11-01: "a phone that shows up in the game at a key... as if it were a real
 * phone"; 2026-11-03: "everything as on the iPhone"), drawn by the plugin over the game's picture while it's out
 * (plugin handset.rs) and used with the mouse while the screen's key is held; also on the player's own phone,
 * from its own QR code (handset/install.ts). An iPhone's home screen (Home.tsx) and its apps. From the same host
 * messages the cab has; nothing of the cab's own doing runs here (the cab does it, once).
 */
export function HandsetApp({ remote }: { remote: boolean }) {
  useEffect(() => connectHost(), [])
  useEffect(() => followTimeZone(), [])
  // The game's language, else (the game not running) the device's: a phone in Portuguese said in English had its
  // browser offer to translate it.
  const language = languageOf(useLive((state) => state.gameLanguage) ?? navigator.language.toLowerCase())
  useEffect(() => {
    document.documentElement.lang = language
  }, [language])
  useLookChanges()
  const prefs = usePrefs()
  // Light or dark as chosen in its Settings, else as the cab is.
  const cab = useLook().appearance
  const appearance = prefs.look === 'auto' ? cab : prefs.look
  const words = WORDS[language === 'pt' ? 'pt' : 'en']
  const [app, setApp] = useState<AppId | null>(null)
  const home = () => setApp(null)
  const compact = app !== null && CAB_APPS.includes(app)

  return (
    <main className={`handset shell appearance-${appearance}${remote ? ' handset-remote' : ''}`}>
      <div className={`handset-screen${app === null ? ` at-home wallpaper-${prefs.wallpaper}` : ''}`}>
        <StatusBar />
        {app === null ? (
          <Home words={words} open={setApp} units={prefs.units} />
        ) : (
          <section className={`ios-app-screen${compact ? ' compact' : ''}`} key={app}>
            <header className="ios-nav">
              <button type="button" className="ios-back" onClick={home}>
                <svg viewBox="0 0 12 20" aria-hidden>
                  <path d="M10 2 2 10l8 8" />
                </svg>
                {words.home}
              </button>
              {compact && <span className="ios-nav-title">{words.apps[app]}</span>}
            </header>
            <div className={`ios-content ios-content-${app}`}>
              {/* Nupay has its own head, the bank's colour. */}
              {!compact && app !== 'nupay' && <h1 className="ios-large-title">{words.apps[app]}</h1>}
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
              {app === 'commands' && <CommandsApp words={COMMAND_WORDS[language === 'pt' ? 'pt' : 'en']} />}
              {app === 'freight' && <FreightApp words={words} units={prefs.units} />}
              {app === 'truck' && <TruckApp words={words} units={prefs.units} />}
              {app === 'settings' && <SettingsApp words={words} prefs={prefs} remote={remote} />}
              {app === 'nupay' && <NupayApp language={language === 'pt' ? 'pt' : 'en'} />}
              {app === 'services' && <ServicesApp words={SERVICES_WORDS[language === 'pt' ? 'pt' : 'en']} />}
            </div>
          </section>
        )}
        <button type="button" className="ios-home-indicator" aria-label={words.home} onClick={home} />
      </div>
    </main>
  )
}

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

/** iOS's status bar: the time in the game, the island, the signal, Wi-Fi and the battery. */
function StatusBar() {
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))
  return (
    <header className="ios-status">
      <span className="ios-status-time">{time === null ? '--:--' : formatGameTime(time)}</span>
      <span className="ios-island" />
      <span className="ios-status-icons" aria-hidden>
        <svg viewBox="0 0 18 12" width="18" height="12">
          <rect x="0" y="8" width="3" height="4" rx="1" />
          <rect x="5" y="5.5" width="3" height="6.5" rx="1" />
          <rect x="10" y="3" width="3" height="9" rx="1" />
          <rect x="15" y="0" width="3" height="12" rx="1" />
        </svg>
        <svg viewBox="0 0 16 12" width="16" height="12">
          <path d="M8 11.5 5.6 9a3.4 3.4 0 0 1 4.8 0Z" />
          <path d="M3.4 6.8a6.5 6.5 0 0 1 9.2 0l-1.4 1.4a4.5 4.5 0 0 0-6.4 0Z" />
          <path d="M1 4.4a9.9 9.9 0 0 1 14 0l-1.4 1.4a7.9 7.9 0 0 0-11.2 0Z" />
        </svg>
        <svg viewBox="0 0 27 13" width="27" height="13">
          <rect x="0.5" y="0.5" width="23" height="12" rx="3.5" fill="none" stroke="currentColor" opacity="0.4" />
          <rect x="2" y="2" width="20" height="9" rx="2" />
          <path d="M25 4.5v4a2 2 0 0 0 0-4Z" opacity="0.4" />
        </svg>
      </span>
    </header>
  )
}

/** What plays, larger, its volume, and the music apps to choose it in. */
function MusicApp({ words, open }: { words: Words; open: (app: AppId) => void }) {
  return (
    <>
      <div className="ios-widget ios-widget-music ios-music-big">
        <NowPlayingWidget />
      </div>
      <Volume words={words} />
      <List header={words.playIn}>
        {(['youtubeMusic', 'spotify'] as const).map((id) => (
          <Row key={id} icon={<AppIcon id={id} small />} label={words.apps[id]} onClick={() => open(id)} />
        ))}
      </List>
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
      <article className="ios-hero">
        <div className="ios-hero-kicker">{job.cargoLoaded ? words.delivering : words.pickingUp}</div>
        <div className="ios-hero-title">{job.cargo}</div>
        <div className="ios-hero-value">$ {job.income.toLocaleString()}</div>
      </article>
      <List>
        <Row
          icon={<RowIcon tone="#8e8e93">{GLYPH_WEIGHT}</RowIcon>}
          label={words.weight}
          value={`${(job.cargoMass / 1000).toLocaleString(undefined, { maximumFractionDigits: 1 })} t`}
        />
        <Row
          icon={<RowIcon tone="#0a84ff">{GLYPH_ROUTE}</RowIcon>}
          label={words.left}
          value={left > 0 ? distance(left, units) : words.noRoute}
        />
        <Row
          icon={<RowIcon tone="#30d158">{GLYPH_CLOCK}</RowIcon>}
          label={words.arrives}
          value={left > 0 ? duration(time / 60) : '—'}
        />
        <Row
          icon={<RowIcon tone={due !== null && due < 0 ? '#ff453a' : '#ff9f0a'}>{GLYPH_CLOCK}</RowIcon>}
          label={words.deadline}
          value={due === null ? '—' : due < 0 ? words.late : duration(due)}
          late={due !== null && due < 0}
        />
        <Row
          icon={<RowIcon tone="#5e5ce6">{GLYPH_ROUTE}</RowIcon>}
          label={words.planned}
          value={distance(job.plannedDistanceKm * 1000, units)}
        />
      </List>
      <List header={words.route}>
        <Row
          icon={<RowIcon tone="#30d158">{GLYPH_PIN}</RowIcon>}
          label={job.special ? job.sourceCity : job.sourceCompany}
          detail={job.special ? words.from : `${words.from} · ${job.sourceCity}`}
        />
        <Row
          icon={<RowIcon tone="#ff453a">{GLYPH_PIN}</RowIcon>}
          label={job.special ? job.destinationCity : job.destinationCompany}
          detail={job.special ? words.to : `${words.to} · ${job.destinationCity}`}
        />
      </List>
    </>
  )
}

/** The truck as the game says it is now: speed, gear, engine, fuel and its range. */
function TruckApp({ words, units }: { words: Words; units: Prefs['units'] }) {
  const truck = useLive((state) => state.truck)
  const t = useLive((state) => state.telemetry)
  if (!t) return <Empty>{words.noGame}</Empty>
  const fuel = units === 'mi' ? `${Math.round(t.fuel / LITRES_PER_GALLON)} gal` : `${Math.round(t.fuel)} L`
  const onOff = (on: boolean) => (on ? words.on : words.off)
  return (
    <>
      <article className="ios-hero">
        <div className="ios-hero-kicker">{[truck?.brand, truck?.name].filter(Boolean).join(' ')}</div>
        <div className="ios-hero-speed">
          {speed(t.speed, units)}
          <span>{speedUnit(units)}</span>
        </div>
        <div className="ios-hero-kicker">
          {words.gear} {formatGear(t.displayedGear)} · {Math.round(t.engineRpm).toLocaleString()} RPM
        </div>
      </article>
      <List header={words.fuel}>
        <Row icon={<RowIcon tone="#ff9f0a">{GLYPH_FUEL}</RowIcon>} label={words.fuel} value={fuel} />
        <Row
          icon={<RowIcon tone="#30d158">{GLYPH_ROUTE}</RowIcon>}
          label={words.range}
          value={distance(t.fuelRange * 1000, units)}
        />
      </List>
      <List header={words.truck}>
        <Row
          icon={<RowIcon tone="#0a84ff">{GLYPH_CLOCK}</RowIcon>}
          label={words.cruise}
          value={t.cruiseControlSpeed > 0 ? `${speed(t.cruiseControlSpeed, units)} ${speedUnit(units)}` : words.off}
        />
        <Row
          icon={<RowIcon tone="#ff453a">{GLYPH_ENGINE}</RowIcon>}
          label={words.engine}
          value={onOff(t.engineEnabled)}
        />
        <Row
          icon={<RowIcon tone="#ff9f0a">{GLYPH_BRAKE}</RowIcon>}
          label={words.parkingBrake}
          value={onOff(t.parkingBrake)}
        />
        <Row
          icon={<RowIcon tone="#5ac8fa">{GLYPH_LIGHT}</RowIcon>}
          label={words.lights}
          value={t.highBeam ? words.high : onOff(t.lowBeam)}
        />
      </List>
    </>
  )
}

const LITRES_PER_GALLON = 3.785411784

/** The phone's look and units (prefs.ts), the volume of what plays, reloading it, and how it's used. */
function SettingsApp({ words, prefs, remote }: { words: Words; prefs: Prefs; remote: boolean }) {
  return (
    <>
      {/* Each choice as wide as its row: its header names it. */}
      <List header={words.look}>
        <Row
          icon={<RowIcon tone="#0a84ff">{GLYPH_LOOK}</RowIcon>}
          label={
            <Segmented
              options={[
                ['auto', words.lookAuto],
                ['light', words.lookLight],
                ['dark', words.lookDark],
              ]}
              chosen={prefs.look}
              choose={(look) => setPrefs({ look })}
            />
          }
        />
      </List>
      <List header={words.units}>
        <Row
          icon={<RowIcon tone="#30d158">{GLYPH_ROUTE}</RowIcon>}
          label={
            <Segmented
              options={[
                ['mi', words.miles],
                ['km', words.kilometres],
              ]}
              chosen={prefs.units}
              choose={(units) => setPrefs({ units })}
            />
          }
        />
      </List>
      <List header={words.wallpaper}>
        <div className="ios-wallpapers">
          {WALLPAPERS.map((wallpaper) => (
            <button
              key={wallpaper}
              type="button"
              className={`ios-wallpaper wallpaper-${wallpaper}${prefs.wallpaper === wallpaper ? ' on' : ''}`}
              onClick={() => setPrefs({ wallpaper })}
            >
              <span>{words.wallpapers[wallpaper]}</span>
            </button>
          ))}
        </div>
      </List>
      <Volume words={words} />
      {/* A new version installed: the page again, without scanning the QR code again (owner 2026-11-02). */}
      <List footer={words.reloadHint}>
        <button type="button" className="ios-row ios-row-action" onClick={() => location.reload()}>
          {words.reload}
        </button>
        {/* On the player's phone: full screen again, once left. */}
        {remote && document.fullscreenEnabled && (
          <button type="button" className="ios-row ios-row-action" onClick={() => void goFullScreen()}>
            {words.fullScreen}
          </button>
        )}
      </List>
      <List header={words.howTo} footer="WestPlay ATS">
        <Row label={<span className="ios-row-wrap">{words.howToText}</span>} />
      </List>
    </>
  )
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="ios-empty">{children}</p>
}

/** Game minutes as "45 min" or "3 h 05". */
function duration(minutes: number): string {
  const whole = Math.max(0, Math.round(minutes))
  return whole < 60 ? `${whole} min` : `${Math.floor(whole / 60)} h ${String(whole % 60).padStart(2, '0')}`
}

const GLYPH_WEIGHT = <path d="M7 9h10l3 11H4ZM9.5 9a2.5 2.5 0 1 1 5 0" />
const GLYPH_ROUTE = (
  <path d="M6 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6Zm12 12a3 3 0 1 0 0 6 3 3 0 0 0 0-6ZM6 9v4a4 4 0 0 0 4 4h5" />
)
const GLYPH_CLOCK = <path d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm-1 4h2v5.4l3.5 2-1 1.7L11 13.6Z" fillRule="evenodd" />
const GLYPH_PIN = (
  <path
    d="M12 2a7 7 0 0 0-7 7c0 5 7 13 7 13s7-8 7-13a7 7 0 0 0-7-7Zm0 4.5a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5Z"
    fillRule="evenodd"
  />
)
const GLYPH_FUEL = <path d="M5 3h9v18H5Zm2 2v5h5V5Zm8 3 3 3v7a1 1 0 0 0 2 0V9l-3-3" />
const GLYPH_ENGINE = <path d="M4 9h3l2-2h5v2h3l2 2h2v6h-2l-2 2H8l-2-2H4Z" />
const GLYPH_BRAKE = (
  <path
    d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm-2 4h3a3 3 0 0 1 0 6h-1v4h-2Zm2 2v2h1a1 1 0 0 0 0-2Z"
    fillRule="evenodd"
  />
)
const GLYPH_LIGHT = <path d="M10 6a6 6 0 0 0 0 12h2V6ZM14 8h7M14 12h7M14 16h7" />
const GLYPH_LOOK = <path d="M12 3a9 9 0 1 0 0 18Zm0 2v14a7 7 0 0 1 0-14Z" fillRule="evenodd" />

const WORDS = {
  en: {
    home: 'Home',
    apps: {
      maps: 'Maps',
      music: 'Music',
      youtubeMusic: 'YT Music',
      spotify: 'Spotify',
      commands: 'Commands',
      freight: 'Job',
      truck: 'Truck',
      settings: 'Settings',
      nupay: 'Nupay',
      services: 'Services',
    } satisfies Record<AppId, string>,
    days: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
    noJob: 'No job right now',
    noGame: 'The game isn’t running',
    delivering: 'Delivering',
    pickingUp: 'Picking up',
    weight: 'Weight',
    left: 'Left on the route',
    arrives: 'Arrives in',
    planned: 'Whole job',
    deadline: 'Deadline',
    late: 'Late',
    route: 'Route',
    from: 'From',
    to: 'To',
    gear: 'Gear',
    fuel: 'Fuel',
    range: 'Range',
    truck: 'Truck',
    cruise: 'Cruise',
    engine: 'Engine',
    parkingBrake: 'Parking brake',
    lights: 'Lights',
    on: 'On',
    off: 'Off',
    high: 'High',
    volume: 'Volume',
    playIn: 'Play in',
    nothingPlays: 'Nothing is playing',
    noRoute: 'No route',
    look: 'Appearance',
    lookAuto: 'Auto',
    lookLight: 'Light',
    lookDark: 'Dark',
    units: 'Units',
    fullScreen: 'Full screen',
    wallpaper: 'Wallpaper',
    wallpapers: { aurora: 'Aurora', ocean: 'Ocean', sunset: 'Sunset' },
    miles: 'Miles',
    kilometres: 'Km',
    howTo: 'How to use',
    reload: 'Reload the page',
    reloadHint: 'After a WestPlay update: loads the new version here, without scanning the QR code again.',
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
      commands: 'Comandos',
      freight: 'Frete',
      truck: 'Caminhão',
      settings: 'Ajustes',
      nupay: 'Nupay',
      services: 'Serviços',
    } satisfies Record<AppId, string>,
    days: ['Segunda-feira', 'Terça-feira', 'Quarta-feira', 'Quinta-feira', 'Sexta-feira', 'Sábado', 'Domingo'],
    noJob: 'Nenhum frete agora',
    noGame: 'O jogo não está aberto',
    delivering: 'Entregando',
    pickingUp: 'Indo buscar',
    weight: 'Peso',
    left: 'Falta na rota',
    arrives: 'Chega em',
    planned: 'Frete inteiro',
    deadline: 'Prazo',
    late: 'Atrasado',
    route: 'Rota',
    from: 'De',
    to: 'Para',
    gear: 'Marcha',
    fuel: 'Diesel',
    range: 'Autonomia',
    truck: 'Caminhão',
    cruise: 'Piloto',
    engine: 'Motor',
    parkingBrake: 'Freio de mão',
    lights: 'Faróis',
    on: 'Ligado',
    off: 'Desligado',
    high: 'Alto',
    volume: 'Volume',
    playIn: 'Tocar no',
    nothingPlays: 'Nada tocando',
    noRoute: 'Sem rota',
    look: 'Aparência',
    lookAuto: 'Auto',
    lookLight: 'Claro',
    lookDark: 'Escuro',
    units: 'Unidades',
    fullScreen: 'Tela cheia',
    wallpaper: 'Papel de parede',
    wallpapers: { aurora: 'Aurora', ocean: 'Oceano', sunset: 'Pôr do sol' },
    miles: 'Milhas',
    kilometres: 'Km',
    howTo: 'Como usar',
    reload: 'Atualizar página',
    reloadHint: 'Depois de uma atualização do WestPlay: carrega a versão nova aqui, sem escanear o QR Code de novo.',
    howToText:
      'Alt+4 tira e guarda o celular. Segure o Alt para mexer nele com o mouse. A tecla pode ser trocada em Controles, no WestPlay.',
  },
}

type Words = (typeof WORDS)['en']

/** The volume of what plays (PhoneVolume.tsx), or that nothing does. */
function Volume({ words }: { words: Words }) {
  const { showing } = useNowPlaying()
  return (
    <List header={words.volume}>
      <Row
        label={
          showing ? <PhoneVolume label={words.volume} /> : <span className="ios-row-off">{words.nothingPlays}</span>
        }
      />
    </List>
  )
}
