import { type CSSProperties, type ReactNode, Suspense, useEffect, useRef, useState } from 'react'
import { formatGameTime, formatGear } from '../shared/format'
import { cabClock } from '../shared/host'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import {
  connectHost,
  useCharacter,
  useCharacterSwitches,
  useLive,
  useFuelBill,
  usePaper,
  useTikTok,
} from '../cabin/state/store'
import { ALERT_GROUPS, type AlertGroup, type CameraNudge, postToHost } from '../shared/host'
import { languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import { PhoneVolume } from '../phone/PhoneVolume'
import { useNowPlaying } from '../cabin/dashboard/useNowPlaying'
import { CharacterApp, CHARACTER_WORDS } from './CharacterApp'
import { CommandsApp } from './CommandsApp'
import { CouplingStep } from './FifthWheel'
import { COMMAND_WORDS } from './commandWords'
import { goFullScreen } from './fullScreen'
import { Home } from './Home'
import { type AppId, AppIcon, RowIcon } from './icons'
import { List, Row, Segmented } from './ios'
import { distance, formatMoney, type Prefs, setPrefs, speed, speedUnit, usePrefs, WALLPAPERS } from './prefs'
import { NupayApp } from './NupayApp'
import { FuelBill } from './FuelBill'
import { Paper } from './Paper'
import { ServicesApp } from './ServicesApp'
import { SERVICES_WORDS } from './servicesWords'
import { Notifications } from './notifications'
import { TikTokApp } from './TikTokApp'
import { SideKeys } from './SideKeys'
import { SuspensionApp, SUSPENSION_WORDS } from './SuspensionApp'
import { FloatScreenApp, FLOAT_SCREEN_WORDS } from './FloatScreenApp'
import { ScrollThumb } from './ScrollThumb'
import { Scaled } from './Scaled'
import { ScreenFitList } from './ScreenFitList'
import { Keyboard, MapView, SpotifyApp, YouTubeMusicApp } from './lazy'
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
  // The app's page, for the scroll bar at its edge (ScrollThumb).
  const content = useRef<HTMLDivElement>(null)
  const home = () => setApp(null)
  // Taken out on one of its apps (its Personagem's keybind): that app open, whatever was.
  const opened = useCharacter((state) => state.opened)
  useEffect(() => {
    if (opened?.app === 'character') setApp('character')
  }, [opened])
  const compact = app !== null && CAB_APPS.includes(app)
  // TikTok's own site shows under the page, through its app (the phone in the game's alone).
  const onTikTok = app === 'tiktok' && !remote
  useTikTokNotifications(prefs, remote)
  useGameNotifications(prefs.alertsOff, remote)
  // The cargo's paper to sign, in place of the phone (the phone in the game's alone: the host shows it large).
  const paper = usePaper((state) => state.stage)
  // The fuel's bill to pay at the pump's attendant, on Nupay, in place of the phone as the paper is.
  const fuelBill = useFuelBill((state) => state.bill)
  if (fuelBill && !remote) {
    return (
      <main className="handset shell is-paper">
        <FuelBill key={fuelBill.id} bill={fuelBill} language={language === 'pt' ? 'pt' : 'en'} />
      </main>
    )
  }
  // The fifth wheel's lock to pull, uncoupling by hand (owner 2026-11-19), shown as the paper is.
  if ((paper === 'lockPull' || paper === 'lockPush' || paper === 'legsUp' || paper === 'legsDown') && !remote) {
    return (
      <main className="handset shell is-paper">
        <CouplingStep key={paper} step={paper} language={language === 'pt' ? 'pt' : 'en'} />
      </main>
    )
  }
  if ((paper === 'pickup' || paper === 'delivery') && !remote) {
    return (
      <main className="handset shell is-paper">
        <Paper key={paper} stage={paper} language={language === 'pt' ? 'pt' : 'en'} prefs={prefs} />
      </main>
    )
  }

  return (
    <main
      className={`handset shell appearance-${appearance}${remote ? ' handset-remote' : ' with-keys'}${onTikTok ? ' on-tiktok' : ''}`}
    >
      {!remote && <SideKeys tiktok={onTikTok} />}
      <div
        className={`handset-screen${app === null ? ` at-home wallpaper-${prefs.wallpaper}` : ''}${onTikTok ? ' on-tiktok' : ''}`}
      >
        <StatusBar />
        <Notifications language={language === 'pt' ? 'pt' : 'en'} currency={prefs.currency} />
        {app === null ? (
          <Home words={words} open={setApp} units={prefs.units} remote={remote} />
        ) : (
          <section
            className={`ios-app-screen${compact ? ' compact' : ''}${app === 'tiktok' ? ' is-tiktok' : ''}`}
            key={app}
          >
            {/* TikTok's is all TikTok's: home is the indicator under it. */}
            {!onTikTok && (
              <header className="ios-nav">
                <button type="button" className="ios-back" onClick={home}>
                  <svg viewBox="0 0 12 20" aria-hidden>
                    <path d="M10 2 2 10l8 8" />
                  </svg>
                  {words.home}
                </button>
                {compact && <span className="ios-nav-title">{words.apps[app]}</span>}
              </header>
            )}
            {!onTikTok && <ScrollThumb target={content} />}
            <div className={`ios-content ios-content-${app}`} ref={content}>
              {/* Nupay has its own head; TikTok in the game is all TikTok's. */}
              {!compact && app !== 'nupay' && !onTikTok && <h1 className="ios-large-title">{words.apps[app]}</h1>}
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
              {app === 'freight' && (
                <FreightApp
                  words={words}
                  units={prefs.units}
                  currency={prefs.currency}
                  language={language === 'pt' ? 'pt' : 'en'}
                />
              )}
              {app === 'truck' && <TruckApp words={words} units={prefs.units} />}
              {app === 'settings' && <SettingsApp words={words} prefs={prefs} remote={remote} />}
              {app === 'nupay' && <NupayApp language={language === 'pt' ? 'pt' : 'en'} currency={prefs.currency} />}
              {app === 'services' && <ServicesApp words={SERVICES_WORDS[language === 'pt' ? 'pt' : 'en']} />}
              {app === 'character' && <CharacterApp words={CHARACTER_WORDS[language === 'pt' ? 'pt' : 'en']} />}
              {app === 'tiktok' && <TikTokApp language={language === 'pt' ? 'pt' : 'en'} remote={remote} />}
              {app === 'suspension' && <SuspensionApp words={SUSPENSION_WORDS[language === 'pt' ? 'pt' : 'en']} />}
              {app === 'camera' && <CameraApp words={words} />}
              {app === 'floatScreen' && (
                <FloatScreenApp
                  words={FLOAT_SCREEN_WORDS[language === 'pt' ? 'pt' : 'en']}
                  language={language === 'pt' ? 'pt' : 'en'}
                />
              )}
            </div>
          </section>
        )}
        <button type="button" className="ios-home-indicator" aria-label={words.home} onClick={home} />
      </div>
    </main>
  )
}

/**
 * The account's TikTok LIVE watched for notifications or not, as the phone's Settings have it: said to the host as
 * the phone in the game's page loads and as it changes. TikTok is run then (hidden), without its app being opened,
 * if it was signed in when last known: a streamer's notifications come from the start.
 */
function useTikTokNotifications(prefs: Prefs, remote: boolean) {
  const signedIn = useTikTok((state) => state.signedIn)
  useEffect(() => {
    if (signedIn !== null && signedIn !== prefs.tiktokSignedIn) setPrefs({ tiktokSignedIn: signedIn })
  }, [signedIn, prefs.tiktokSignedIn])
  const wanted = prefs.tiktokNotifications
  const known = prefs.tiktokSignedIn
  // Only while the game runs (its telemetry comes): the host closes TikTok with the game, as its players.
  const playing = useLive((state) => state.telemetry !== null)
  useEffect(() => {
    if (remote) return
    if (wanted && known && playing) postToHost({ type: 'tiktok', command: { action: 'start' } })
    postToHost({ type: 'tiktok', command: { action: 'notifications', on: wanted } })
    // Said again as TikTok starts (`signedIn` known): one started since heard nothing of it.
  }, [remote, wanted, known, playing, signedIn])
}

/**
 * The game's notifications' groups turned off in the phone's Settings, said to the host (it's the host that tells
 * them, and shows the phone's top for them) as the phone in the game's page loads and as they change.
 */
function useGameNotifications(off: AlertGroup[], remote: boolean) {
  const said = off.join(',')
  useEffect(() => {
    if (remote) return
    postToHost({ type: 'alerts', off: said ? (said.split(',') as AlertGroup[]) : [] })
  }, [remote, said])
}

/** One of the cab's own screens, smaller (Scaled.tsx), in an app window as the cab has it, with the cab's
 * keyboard for its search fields (the map's, the music apps'). */
function CabScreen({ scale, children }: { scale: number; children: ReactNode }) {
  return (
    <Scaled scale={scale} className="handset-cab">
      <div className="screen app-window" style={{ '--content-left': '16px', '--edge': '16px' } as CSSProperties}>
        {/* Its parts loaded as it opens (lazy.tsx): nothing until then. */}
        <Suspense fallback={null}>
          {children}
          <Keyboard />
        </Suspense>
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

/** The job, as the game gives it: the cargo and its weight, from where to where, how far yet, what it pays (in the
 * currency chosen in Nupay, as its money: prefs.ts `formatMoney`). */
function FreightApp({
  words,
  units,
  currency,
  language,
}: {
  words: Words
  units: Prefs['units']
  currency: Prefs['currency']
  language: 'en' | 'pt'
}) {
  const job = useLive((state) => state.job)
  // The pay is in the game's money (ATS's dollars, ETS2's euros), as the account's (Nupay's, the same way).
  const gameCurrency = useLive((state) => state.wallet?.currency ?? 'USD')
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
        <div className="ios-hero-value">{formatMoney(job.income, gameCurrency, currency, language)}</div>
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

/**
 * The reversing camera's own app (owner 2026-11-20: at the end of the Truck page "it isn't showing for me … make an
 * app"): big arrows moving it as its cab screen's buttons do, how far it has moved, and back to where it goes by
 * itself. For the trailer coupled now, or the truck alone (rear_camera.rs `nudged`).
 */
function CameraApp({ words }: { words: Words }) {
  const t = useLive((state) => state.telemetry)
  if (!t) return <Empty>{words.noGame}</Empty>
  const press = (action: CameraNudge) => postToHost({ type: 'cameraNudge', action })
  const language = words === WORDS.pt ? 'pt' : 'en'
  const back = t.cameraBack ?? 0
  const up = t.cameraUp ?? 0
  return (
    <>
      <article className="ios-hero ios-camera-hero">
        <div className="ios-hero-kicker">{t.trailerConnected ? words.cameraForTrailer : words.cameraForTruck}</div>
        <div className="ios-camera-pad">
          <button type="button" className="ios-camera-key up" onClick={() => press('up')}>
            <span aria-hidden>▲</span>
            {words.cameraUp}
          </button>
          <button type="button" className="ios-camera-key forward" onClick={() => press('forward')}>
            <span aria-hidden>◀</span>
            {words.cameraForward}
          </button>
          <div className="ios-camera-now">
            <div>
              {words.cameraDistance} <b>{signedMetres(back, language)}</b>
            </div>
            <div>
              {words.cameraHeight} <b>{signedMetres(up, language)}</b>
            </div>
          </div>
          <button type="button" className="ios-camera-key back" onClick={() => press('back')}>
            {words.cameraBack}
            <span aria-hidden>▶</span>
          </button>
          <button type="button" className="ios-camera-key down" onClick={() => press('down')}>
            {words.cameraDown}
            <span aria-hidden>▼</span>
          </button>
        </div>
      </article>
      {/* Turned too (owner 2026-11-20: "turn it up if it looks too far down; to the sides; a wider view"). */}
      <List header={words.cameraTurning}>
        <CameraTurnRow
          name={words.cameraTilt}
          value={signedDegrees(t.cameraTilt ?? 0, language)}
          less={{ label: words.cameraTiltDown, press: () => press('tiltDown') }}
          more={{ label: words.cameraTiltUp, press: () => press('tiltUp') }}
        />
        <CameraTurnRow
          name={words.cameraTurn}
          value={signedDegrees(t.cameraTurn ?? 0, language)}
          less={{ label: words.cameraTurnLeft, press: () => press('turnLeft') }}
          more={{ label: words.cameraTurnRight, press: () => press('turnRight') }}
        />
        <CameraTurnRow
          name={words.cameraWide}
          value={signedDegrees(t.cameraWide ?? 0, language)}
          less={{ label: words.cameraNarrower, press: () => press('narrower') }}
          more={{ label: words.cameraWider, press: () => press('wider') }}
        />
      </List>
      <List footer={words.cameraNote}>
        <button type="button" className="ios-row ios-row-action" onClick={() => press('reset')}>
          {words.cameraReset}
        </button>
      </List>
    </>
  )
}

/** One of the camera's turns: its name and how far it's turned, and a button each way. */
function CameraTurnRow({
  name,
  value,
  less,
  more,
}: {
  name: string
  value: string
  less: { label: string; press: () => void }
  more: { label: string; press: () => void }
}) {
  return (
    <div className="ios-row ios-camera-row">
      <span className="ios-camera-name">
        {name} <b>{value}</b>
      </span>
      <span className="ios-camera-buttons">
        <button type="button" onClick={less.press}>
          {less.label}
        </button>
        <button type="button" onClick={more.press}>
          {more.label}
        </button>
      </span>
    </div>
  )
}

/** A signed angle: "+4°". */
function signedDegrees(degrees: number, language: 'en' | 'pt'): string {
  const sign = degrees > 0 ? '+' : degrees < 0 ? '−' : ''
  const number = String(Math.round(Math.abs(degrees) * 10) / 10)
  return `${sign}${language === 'pt' ? number.replace('.', ',') : number}°`
}

/** A signed distance: "+0,50 m". */
function signedMetres(metres: number, language: 'en' | 'pt'): string {
  const sign = metres > 0 ? '+' : metres < 0 ? '−' : ''
  const number = Math.abs(metres).toFixed(2)
  return `${sign}${language === 'pt' ? number.replace('.', ',') : number} m`
}

const LITRES_PER_GALLON = 3.785411784

/** The phone's look and units (prefs.ts), the volume of what plays, reloading it, and how it's used. */
function SettingsApp({ words, prefs, remote }: { words: Words; prefs: Prefs; remote: boolean }) {
  const switches = useCharacterSwitches()
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
      <List header={words.currency}>
        <Row
          icon={<RowIcon tone="#8a05be">{GLYPH_MONEY}</RowIcon>}
          label={
            <Segmented
              options={[
                ['auto', words.currencyGame],
                ['USD', 'US$'],
                ['BRL', 'R$'],
                ['EUR', '€'],
              ]}
              chosen={prefs.currency}
              choose={(currency) => setPrefs({ currency })}
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
      {/* The account's TikTok LIVE as notifications: the phone in the game's (it's the PC that watches it). */}
      {!remote && (
        <List header={words.tiktokNotifications} footer={words.tiktokNotificationsHint}>
          <Row
            icon={<AppIcon id="tiktok" small />}
            label={
              <Segmented
                options={[
                  ['on', words.on],
                  ['off', words.off],
                ]}
                chosen={prefs.tiktokNotifications ? 'on' : 'off'}
                choose={(chosen) => setPrefs({ tiktokNotifications: chosen === 'on' })}
              />
            }
          />
          <button
            type="button"
            className="ios-row ios-row-action"
            onClick={() => postToHost({ type: 'tiktok', command: { action: 'testNotification' } })}
          >
            {words.testNotification}
          </button>
        </List>
      )}
      {/* The game's notifications (owner 2026-11-20: payments, fines, the truck, the job), each group on or off. */}
      {!remote && (
        <List header={words.gameNotifications} footer={words.gameNotificationsHint}>
          {ALERT_GROUPS.map((group) => (
            <Row
              key={group}
              icon={ALERT_ICONS[group]}
              label={
                <span className="ios-alert-row">
                  <span>{words.alertGroups[group]}</span>
                  <Segmented
                    options={[
                      ['on', words.on],
                      ['off', words.off],
                    ]}
                    chosen={prefs.alertsOff.includes(group) ? 'off' : 'on'}
                    choose={(chosen) =>
                      setPrefs({
                        alertsOff:
                          chosen === 'on'
                            ? prefs.alertsOff.filter((each) => each !== group)
                            : [...prefs.alertsOff.filter((each) => each !== group), group],
                      })
                    }
                  />
                </span>
              }
            />
          ))}
          <button type="button" className="ios-row ios-row-action" onClick={() => postToHost({ type: 'testAlerts' })}>
            {words.testGameNotifications}
          </button>
        </List>
      )}
      <SwitchList
        header={words.keepScreenOn}
        footer={words.keepScreenOnHint}
        icon={<RowIcon tone="#0a84ff">{GLYPH_SCREEN}</RowIcon>}
        on={switches.keepScreenOn}
        change={(on) => postToHost({ type: 'keepScreenOn', on })}
        words={words}
      />
      {/* The Modo Personagem's switches: the app's settings (owner 2026-11-13: in the app and on the phone too),
          the same here, on the player's own phone and on the companion's Personagem page. */}
      <SwitchList
        header={words.freightPapers}
        footer={words.freightPapersHint}
        icon={<RowIcon tone="#1f8f4a">{GLYPH_PAPER}</RowIcon>}
        on={switches.freightPapers}
        change={(on) => postToHost({ type: 'freightPapers', on })}
        words={words}
      />
      <SwitchList
        header={words.manualRefuel}
        footer={words.manualRefuelHint}
        icon={<RowIcon tone="#c9541a">{GLYPH_FUEL}</RowIcon>}
        on={switches.manualRefuel}
        change={(on) => postToHost({ type: 'manualRefuel', on })}
        words={words}
      />
      <SwitchList
        header={words.payAttendant}
        footer={words.payAttendantHint}
        icon={<RowIcon tone="#820ad1">{GLYPH_FUEL}</RowIcon>}
        on={switches.payAttendant}
        change={(on) => postToHost({ type: 'payAttendant', on })}
        words={words}
      />
      {/* Which of the game's virtual mirrors show (owner 2026-11-18): the reversing camera uses the left one, shown
          or not. On the player's own phone too. */}
      <List header={words.virtualMirrors} footer={words.virtualMirrorsHint}>
        <Row
          icon={<RowIcon tone="#0a84ff">{GLYPH_MIRROR}</RowIcon>}
          label={
            <Segmented
              options={[
                ['none', words.mirrorsNone],
                ['left', words.mirrorsLeft],
                ['right', words.mirrorsRight],
                ['both', words.mirrorsBoth],
              ]}
              chosen={switches.virtualMirrors}
              choose={(which) => postToHost({ type: 'virtualMirrors', which })}
            />
          }
        />
      </List>
      {/* How big the mouse's dot is, on the truck's screen and the phone (owner 2026-11-19). */}
      <List header={words.cursorSize} footer={words.cursorSizeHint}>
        <Row
          icon={<RowIcon tone="#5e5ce6">{GLYPH_CURSOR}</RowIcon>}
          label={
            <Segmented
              options={[
                ['small', words.cursorSmall],
                ['normal', words.cursorNormal],
                ['large', words.cursorLarge],
                ['larger', words.cursorLarger],
              ]}
              chosen={switches.cursorSize}
              choose={(which) => postToHost({ type: 'cursorSize', which })}
            />
          }
        />
      </List>
      <ScreenFitList words={words} />
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
      <List header={words.howTo} footer="MaxDevMod ATS">
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
const GLYPH_MONEY = (
  <path
    d="M3 6h18v12H3Zm9 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6ZM6 9v6M18 9v6"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
  />
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
const GLYPH_PAPER = (
  <path d="M6 2h9l4 4v16H6Zm8 1.5V7h3.5ZM9 11h7v1.6H9Zm0 3.4h7V16H9Zm0 3.4h4.5v1.6H9Z" fillRule="evenodd" />
)
const GLYPH_CURSOR = (
  <>
    <circle cx="12" cy="12" r="7" fill="none" stroke="currentColor" strokeWidth="2" />
    <circle cx="12" cy="12" r="3" />
  </>
)
const GLYPH_MIRROR = (
  <path
    d="M4 6a3 3 0 0 1 3-3h10a3 3 0 0 1 3 3v8a3 3 0 0 1-3 3h-4v3h-2v-3H7a3 3 0 0 1-3-3Zm3-1a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1Z"
    fillRule="evenodd"
  />
)
/** The game's notifications' groups in Settings, each with its glyph. */
const ALERT_ICONS: Record<AlertGroup, ReactNode> = {
  moneyIn: <RowIcon tone="#8a05be">{GLYPH_MONEY}</RowIcon>,
  moneyOut: <RowIcon tone="#5e2a84">{GLYPH_MONEY}</RowIcon>,
  fine: <RowIcon tone="#ff3b30">{<path d="M12 2 1 21h22Zm-1 7h2v6h-2Zm0 8h2v2h-2Z" />}</RowIcon>,
  fuel: <RowIcon tone="#c9541a">{GLYPH_FUEL}</RowIcon>,
  damage: <RowIcon tone="#1c7ed6">{GLYPH_ENGINE}</RowIcon>,
  rest: <RowIcon tone="#5856d6">{GLYPH_CLOCK}</RowIcon>,
  job: <RowIcon tone="#ff9500">{GLYPH_PAPER}</RowIcon>,
}
const GLYPH_LOOK = <path d="M12 3a9 9 0 1 0 0 18Zm0 2v14a7 7 0 0 1 0-14Z" fillRule="evenodd" />
const GLYPH_SCREEN = (
  <>
    <rect x="3" y="5" width="18" height="12" rx="2" fill="none" stroke="currentColor" strokeWidth="2" />
    <path d="M8 20h8M12 17v3" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
  </>
)

const WORDS = {
  en: {
    home: 'Home',
    pageBack: 'Previous page',
    pageNext: 'Next page',
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
      character: 'Character',
      tiktok: 'TikTok',
      suspension: 'Suspension',
      camera: 'Rear Camera',
      floatScreen: 'Screen',
    } satisfies Record<AppId, string>,
    days: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
    noJob: 'No job right now',
    noGame: 'The game isn’t running',
    openMap: 'Tap to open the map',
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
    camera: 'Reversing camera',
    cameraNote:
      'Moves and turns the camera for the trailer coupled now (or the truck alone): 5 cm, 1° up or down, 2° to the side or 5° of view a press. Kept for it; best tried in reverse, watching the cab screen.',
    cameraDistance: 'Distance',
    cameraHeight: 'Height',
    cameraForward: 'Forward',
    cameraBack: 'Back',
    cameraDown: 'Lower',
    cameraUp: 'Higher',
    cameraReset: 'Back to default',
    cameraForTrailer: 'Adjusting for the trailer coupled now',
    cameraForTruck: 'Adjusting for the truck alone',
    cameraTurning: 'Turn and view',
    cameraTilt: 'Tilt',
    cameraTiltUp: 'Up',
    cameraTiltDown: 'Down',
    cameraTurn: 'Turn',
    cameraTurnLeft: 'Left',
    cameraTurnRight: 'Right',
    cameraWide: 'Field of view',
    cameraWider: 'Wider',
    cameraNarrower: 'Narrower',
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
    currency: 'Nupay currency',
    currencyGame: 'Game',
    fullScreen: 'Full screen',
    wallpaper: 'Wallpaper',
    tiktokNotifications: 'TikTok LIVE notifications',
    tiktokNotificationsHint:
      'While you’re live on TikTok, who follows, sends a gift, shares or subscribes shows on the phone, with a sound. Put away, the top of the phone comes up for it. Sign in to TikTok under Accounts, in MaxDevMod.',
    testNotification: 'Test a notification',
    gameNotifications: 'Game notifications',
    gameNotificationsHint:
      'Payments, fines, the truck and the job show on the phone, with a sound. Put away, the top of the phone comes up for them.',
    testGameNotifications: 'Test the game’s notifications',
    alertGroups: {
      moneyIn: 'Payments received',
      moneyOut: 'Payments made (tolls, fuel…)',
      fine: 'Fines',
      fuel: 'Fuel (low, pay the attendant)',
      damage: 'Truck damage',
      rest: 'Rest (fatigue)',
      job: 'Jobs and deadlines',
    } satisfies Record<AlertGroup, string>,
    freightPapers: 'Cargo papers and coupling by hand',
    freightPapersHint:
      'On a job with your own truck: get out, sign the cargo’s bill with the clerk by the trailer, then couple the trailer at the fifth wheel (E). At the other end, the recipient signs before you can deliver. Needs the Modo Personagem.',
    manualRefuel: 'Refuelling by hand',
    manualRefuelHint:
      'At a pump, the refuel key in the cab no longer fills the tank: get out, walk to the truck’s tank (beside it, behind the cab) and press E. E again stops. Needs the Modo Personagem.',
    virtualMirrors: 'Virtual mirrors',
    virtualMirrorsHint:
      'Which of the game’s virtual mirrors show on the screen. The reversing camera works with any of them: it’s made from the left one, kept on (hidden if not chosen).',
    mirrorsNone: 'None',
    mirrorsLeft: 'Left',
    mirrorsRight: 'Right',
    mirrorsBoth: 'Both',
    cursorSize: 'Mouse dot',
    cursorSizeHint: 'How big the dot that follows the mouse is, on the truck’s screen and on the phone.',
    cursorSmall: 'Small',
    cursorNormal: 'Normal',
    cursorLarge: 'Large',
    cursorLarger: 'Larger',
    payAttendant: 'Pay the pump attendant',
    payAttendantHint:
      'After refuelling by hand, the fuel is owed: walk up to the attendant in front of the truck and press E to pay on Nupay. Off, it’s paid at once.',
    keepScreenOn: 'Screen on with truck off',
    keepScreenOnHint:
      'Keeps the dashboard multimedia screen powered on and interactive even when the truck is turned off. Toggle off anytime to turn it back off.',
    wallpapers: { aurora: 'Aurora', ocean: 'Ocean', sunset: 'Sunset' },
    miles: 'Miles',
    kilometres: 'Km',
    howTo: 'How to use',
    reload: 'Reload the page',
    reloadHint: 'After a MaxDevMod update: loads the new version here, without scanning the QR code again.',
    screenFit: 'Vehicle screen',
    screenFitHint:
      'Fits MaxDevMod on the screen of the vehicle you’re in, there at once: for when it’s too small or cut off to reach its own Settings.',
    screenFitNone: 'Get in a vehicle with MaxDevMod on its screen.',
    edgeTop: 'Top edge',
    edgeBottom: 'Bottom edge',
    edgeLeft: 'Left edge',
    edgeRight: 'Right edge',
    proportion: 'Proportion',
    fitSize: 'Size',
    restoreFit: 'Restore default',
    fitPresets: {
      title: 'Screen format (Manual)',
      auto: 'Automatic (Detected)',
      widescreen: '16:9 Widescreen',
      sixteenTen: '16:10 Medium',
      classic: '4:3 Classic',
      ultrawide: '21:9 Ultrawide',
      bandCenter: 'Mod Square Band',
    },
    howToText:
      'Alt+4 takes the phone out and puts it away. Hold Alt to use it with the mouse. The key can be changed in MaxDevMod’s Controls.',
  },
  pt: {
    home: 'Início',
    pageBack: 'Página anterior',
    pageNext: 'Próxima página',
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
      character: 'Personagem',
      tiktok: 'TikTok',
      suspension: 'Suspensão',
      camera: 'Câmera de Ré',
      floatScreen: 'Tela',
    } satisfies Record<AppId, string>,
    days: ['Segunda-feira', 'Terça-feira', 'Quarta-feira', 'Quinta-feira', 'Sexta-feira', 'Sábado', 'Domingo'],
    noJob: 'Nenhum frete agora',
    noGame: 'O jogo não está aberto',
    openMap: 'Toque para abrir o mapa',
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
    camera: 'Câmera de ré',
    cameraNote:
      'Move e gira a câmera da carreta engatada agora (ou do caminhão sozinho): 5 cm, 1° para cima ou para baixo, 2° para o lado ou 5° de visão por toque. Fica guardado para ela; melhor ajustar de ré, olhando a tela da cabine.',
    cameraDistance: 'Distância',
    cameraHeight: 'Altura',
    cameraForward: 'Para frente',
    cameraBack: 'Para trás',
    cameraDown: 'Descer',
    cameraUp: 'Subir',
    cameraReset: 'Voltar ao padrão',
    cameraForTrailer: 'Ajustando para a carreta engatada',
    cameraForTruck: 'Ajustando para o caminhão sozinho',
    cameraTurning: 'Girar e campo de visão',
    cameraTilt: 'Inclinação',
    cameraTiltUp: 'Para cima',
    cameraTiltDown: 'Para baixo',
    cameraTurn: 'Girar',
    cameraTurnLeft: 'Esquerda',
    cameraTurnRight: 'Direita',
    cameraWide: 'Campo de visão',
    cameraWider: 'Mais',
    cameraNarrower: 'Menos',
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
    currency: 'Moeda do Nupay',
    currencyGame: 'Do jogo',
    fullScreen: 'Tela cheia',
    wallpaper: 'Papel de parede',
    tiktokNotifications: 'Notificações da LIVE do TikTok',
    tiktokNotificationsHint:
      'Enquanto você faz live no TikTok, quem segue, manda presente, compartilha ou se inscreve aparece no celular, com som. Guardado, o topo do celular sobe para mostrar. Entre no TikTok em Contas, no MaxDevMod.',
    testNotification: 'Testar notificação',
    gameNotifications: 'Notificações do jogo',
    gameNotificationsHint:
      'Pagamentos, multas, o caminhão e o frete aparecem no celular, com som. Guardado, o topo do celular sobe para mostrar.',
    testGameNotifications: 'Testar notificações do jogo',
    alertGroups: {
      moneyIn: 'Pagamentos recebidos',
      moneyOut: 'Pagamentos feitos (pedágio, diesel…)',
      fine: 'Multas',
      fuel: 'Combustível (reserva, pagar frentista)',
      damage: 'Dano no caminhão',
      rest: 'Descanso (sono)',
      job: 'Fretes e prazos',
    } satisfies Record<AlertGroup, string>,
    freightPapers: 'Nota fiscal e engate manual',
    freightPapersHint:
      'Nos fretes com o seu caminhão: desça, assine a nota fiscal com o conferente ao lado da carreta e engate a carreta na quinta roda (E). Na entrega, o destinatário assina antes de você poder entregar. Precisa do Modo Personagem.',
    manualRefuel: 'Abastecimento manual',
    manualRefuelHint:
      'Na bomba, a tecla de abastecer na cabine não enche mais o tanque: desça, vá até o tanque do caminhão (ao lado dele, atrás da cabine) e aperte E. E de novo para. Precisa do Modo Personagem.',
    virtualMirrors: 'Espelhos virtuais',
    virtualMirrorsHint:
      'Quais espelhos virtuais do jogo aparecem na tela. A câmera de ré funciona com qualquer escolha: ela é feita do espelho esquerdo, que fica ligado (escondido, se não for escolhido).',
    mirrorsNone: 'Nenhum',
    mirrorsLeft: 'Esquerdo',
    mirrorsRight: 'Direito',
    mirrorsBoth: 'Os dois',
    cursorSize: 'Bolinha do mouse',
    cursorSizeHint: 'O tamanho da bolinha que segue o mouse, na tela do caminhão e no celular.',
    cursorSmall: 'Pequena',
    cursorNormal: 'Normal',
    cursorLarge: 'Grande',
    cursorLarger: 'Muito grande',
    payAttendant: 'Pagar no frentista',
    payAttendantHint:
      'Depois de abastecer a pé, o abastecimento fica em aberto: vá até o frentista na frente do caminhão e aperte E para pagar no Nupay. Desligado, já fica pago.',
    keepScreenOn: 'Tela ligada (caminhão desligado)',
    keepScreenOnHint:
      'Mantém a central multimídia ligada e funcionando mesmo com o caminhão desligado. Desative quando quiser desligar a tela junto com o caminhão.',
    wallpapers: { aurora: 'Aurora', ocean: 'Oceano', sunset: 'Pôr do sol' },
    miles: 'Milhas',
    kilometres: 'Km',
    howTo: 'Como usar',
    reload: 'Atualizar página',
    reloadHint: 'Depois de uma atualização do MaxDevMod: carrega a versão nova aqui, sem escanear o QR Code de novo.',
    screenFit: 'Tela do veículo',
    screenFitHint:
      'Ajusta o MaxDevMod na tela do veículo em que você está, na hora: para quando ela fica pequena ou cortada demais para chegar nos Ajustes dela.',
    screenFitNone: 'Entre num veículo com o MaxDevMod na tela.',
    edgeTop: 'Borda de cima',
    edgeBottom: 'Borda de baixo',
    edgeLeft: 'Borda esquerda',
    edgeRight: 'Borda direita',
    proportion: 'Proporção',
    fitSize: 'Tamanho',
    restoreFit: 'Restaurar padrão',
    fitPresets: {
      title: 'Formato da tela (Manual)',
      auto: 'Automático (Detectado)',
      widescreen: '16:9 Widescreen',
      sixteenTen: '16:10 Médio',
      classic: '4:3 Clássico',
      ultrawide: '21:9 Ultrawide',
      bandCenter: 'Faixa Central (Mod)',
    },
    howToText:
      'Alt+4 tira e guarda o celular. Segure o Alt para mexer nele com o mouse. A tecla pode ser trocada em Controles, no MaxDevMod.',
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

/** A Modo Personagem switch in Settings: on or off, as the app's settings have it (`useCharacterSwitches`). */
function SwitchList({
  header,
  footer,
  icon,
  on,
  change,
  words,
}: {
  header: string
  footer: string
  icon: ReactNode
  on: boolean
  change: (on: boolean) => void
  words: { on: string; off: string }
}) {
  return (
    <List header={header} footer={footer}>
      <Row
        icon={icon}
        label={
          <Segmented
            options={[
              ['on', words.on],
              ['off', words.off],
            ]}
            chosen={on ? 'on' : 'off'}
            choose={(chosen) => change(chosen === 'on')}
          />
        }
      />
    </List>
  )
}
