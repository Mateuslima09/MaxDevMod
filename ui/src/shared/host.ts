import type { CustomWallpaper, IconSet } from './customize'

/** Live screen sharpness tier applied in D3D11 on the next frame (without restarting the game). */
export type ScreenResolution = 'gameDefault' | 'standard' | 'high' | 'ultra'

/**
 * Messages from the WestPlay host to the cabin page (WebView2 web messages). Mirrors the Rust side:
 * `westplay_shared::Telemetry`, `westplay_shared::TruckInfo`, the screen layout the game draws WestPlay on
 * (an id from crates/shared/screen_layouts.txt), and the app's dev commands.
 */

export interface Telemetry {
  x: number
  y: number
  z: number
  /** Turns (0..1), 0 = north. */
  heading: number
  pitch: number
  roll: number
  /** m/s, negative when reversing. */
  speed: number
  engineRpm: number
  gear: number
  displayedGear: number
  cruiseControlSpeed: number
  /** Litres. */
  fuel: number
  /** km. */
  fuelRange: number
  /** m / s / m/s (0 = no limit known). */
  navigationDistance: number
  navigationTime: number
  navigationSpeedLimit: number
  /** Minutes since day 0, 00:00 (day 0 is a Monday): the player's own time, which a job's times are in. */
  gameTime: number
  /** In a Convoy session, the session's time less the player's own, in-game minutes (0 out of one): the cab's
   * clock shows `gameTime` plus this ([`cabClock`]). */
  multiplayerTimeOffset: number
  localScale: number
  dashboardBacklight: number
  electricEnabled: boolean
  engineEnabled: boolean
  parkingBrake: boolean
  lowBeam: boolean
  highBeam: boolean
  leftBlinker: boolean
  rightBlinker: boolean
  paused: boolean
  /** The steering as the truck has it (-1 to 1, left positive). */
  steering: number
  /** The reversing camera's (cabin/reversing): the steered wheels' turn (radians, left positive), and the truck
   * laid out in its own space (metres, z back): its steered axle, the axle it turns about, its fifth wheel, its
   * rear end and its half width (0 until known). */
  wheelSteering: number
  frontAxleZ: number
  rearAxleZ: number
  hookZ: number
  rearZ: number
  halfWidth: number
  /** The first trailer, coupled: where it is and how it's turned (turns), and laid out in its own space (its
   * kingpin, its axle, its rear end, its half width). */
  trailerConnected: boolean
  trailerX: number
  trailerY: number
  trailerZ: number
  trailerHeading: number
  trailerHookZ: number
  trailerAxleZ: number
  trailerRearZ: number
  trailerHalfWidth: number
  /** The parking sensors through the game's own physics (crates/plugin ray.rs): each rear sensor's nearest thing
   * (metres, in cabin/reversing/sensor.ts SENSORS' order; -1: nothing in range, or not known), and how the rays
   * are (`rays`: 0 not yet, 1 working, 2 off after the game closed while they were tested, 3 not available). */
  raySensors: number[]
  rays: number
  /** Whether the game drew its rear-view mirrors lately (crates/plugin mirror.rs): the reversing camera shows them. */
  mirror: boolean
  /** Whether the plugin's reversing camera is settled and can be shown (the page covers the screen till then). */
  cameraReady?: boolean
  /** The reversing camera as the player moved it for this truck or trailer (metres: further back, higher, sideways). */
  cameraBack?: number
  cameraUp?: number
  cameraSide?: number
  /** And turned (degrees: further up, to the right, how much wider, lateral roll: rear_camera.rs `nudge`). */
  cameraTilt?: number
  cameraTurn?: number
  cameraWide?: number
  cameraRoll?: number
  /** MaxDevMod's floating screen (float_screen.rs): switched on, drawn lately (the camera in the cab), and its move in
   * this truck (metres right, up, forward; degrees turned right, tilted up, rolled right). Missing from an older plugin. */
  floatOn?: boolean
  floatSeen?: boolean
  floatPose?: [number, number, number, number, number, number]
  /** How far the springs are pressed in (metres): the truck's front axle, its rear axles, the trailer's; -1 not known
   * (the phone's Suspensão app). Missing from an older plugin. */
  suspension?: [number, number, number]
}

/** The time the cab's clock shows (minutes since day 0, 00:00): the game's, or in a Convoy session the
 * session's, shifted by `shift` minutes, the time zone where the truck is with the profile's time zones on
 * (the cab's `clockShift`, cabin/state/timeZone.ts: an hour on in Finland, an hour back in the UK).
 * Everything shown as a time goes by it; whether a job is late, by `gameTime`. */
export function cabClock(telemetry: Telemetry, shift: number): number {
  // A telemetry without it (none sent yet) is out of a session.
  return telemetry.gameTime + (telemetry.multiplayerTimeOffset || 0) + shift
}

export interface TruckInfo {
  /** e.g. "daf" */
  brandId: string
  /** Display brand, localized by the game, e.g. "DAF". */
  brand: string
  /** Model id, e.g. "vehicle.daf.2021". */
  id: string
  /** Display model name, localized by the game, e.g. "XG+". */
  name: string
}

/** The currency a game counts its money in (crates/host wallet.rs): ATS's dollars, ETS2's euros. */
export type GameCurrency = 'USD' | 'EUR'

/** One sum in or out of the player's account (crates/host wallet.rs `Entry`; an expense negative): what for, about
 * what, and when (the PC's time, ms; the game's, minutes, when known). */
export interface WalletLoan {
  index?: number
  amount: number
  originalAmount: number
  paidAmount?: number
  interestRate: number
  duration: number
  durationWeeks?: number
  dailyPayment: number
  dailyPrincipal?: number
  daysTotal?: number
  daysPaid?: number
  daysRemaining?: number
  totalRepayment?: number
  canRepay?: boolean
}

export interface WalletLoanOffer {
  index: number
  amount: number
  interestRate: number
  durationWeeks: number
  durationDays: number
  dailyPayment: number
  totalRepayment: number
  canTake: boolean
}

export interface PendingLoan {
  amount: number
  days: number
  rate: number
}

export interface WalletEntry {
  at: number
  gameTime: number | null
  kind: 'delivered' | 'cancelled' | 'fined' | 'toll' | 'ferry' | 'train' | 'fuel' | 'loan' | 'loanRepay' | 'other'
  amount: number
  detail: string
}

export type BankAnswer =
  | 'ok'
  | 'unsupported'
  | 'multiplayer'
  | 'locked'
  | 'limitExceeded'
  | 'insufficientFunds'
  | 'invalidIndex'
  | 'notReady'
  | 'failed'
  | 'noAnswer'

export type FleetAnswer =
  | 'ok'
  | 'unsupported'
  | 'multiplayer'
  | 'onJob'
  | 'alreadyActive'
  | 'insufficientFunds'
  | 'alreadyMax'
  | 'notFound'
  | 'notReady'
  | 'failed'
  | 'noAnswer'

export interface FleetGarage {
  garagePtr: string
  cityId: string
  city: string
  tier: 'tiny' | 'small' | 'large'
  slots: number
  usedTrucks: number
  usedDrivers: number
  trailersCount: number
  productivity: number
  upgradeCost?: number
  nextSlots?: number
  canUpgrade: boolean
  /** Per-bay state: 0 = empty, 1 = AI/parked truck, 2 = player truck. */
  slotStates: number[]
}

export interface FleetTruck {
  truckPtr: string
  brand: string
  model: string
  engine: string
  chassis: string
  plate: string
  plateCountry: string
  garageCityId: string
  garageCity: string
  odometerKm: number
  wearPct: number
  slotIndex: number
  isPlayer: boolean
  hasAiDriver: boolean
}

export interface FleetTrailer {
  trailerPtr: string
  brand: string
  model: string
  body: string
  bodyType: string
  chainType: string
  plate: string
  plateCountry: string
  garageCityId: string
  garageCity: string
  volume: number
  grossWeight: number
  axles: number
  wearPct: number
  isAttached: boolean
}

export interface FleetSnapshot {
  gameTime: number
  balance: number
  onJob: boolean
  hasTruck: boolean
  hasAttachedTrailer: boolean
  garages: FleetGarage[]
  trucks: FleetTruck[]
  trailers: FleetTrailer[]
}

/** The fuel filled by hand at a pump (`westplay_shared::FuelBill`): how much (litres, or gallons), its price and
 * total in the money the pump showed (`sign`). */
export interface FuelBill {
  id: number
  gallons: boolean
  amount: number
  price: number
  total: number
  sign: string
}

/** The player's job (`westplay_shared::JobInfo`): names localized by the game. */
export interface JobInfo {
  /** There is a job. */
  active: boolean
  /** The cargo is on: delivering it (otherwise on the way to pick it up). */
  cargoLoaded: boolean
  /** A special transport (no companies). */
  special: boolean
  cargo: string
  /** kg. */
  cargoMass: number
  sourceCity: string
  sourceCompany: string
  destinationCity: string
  destinationCompany: string
  /** When the delivery window ends: minutes since day 0, 00:00 (as `Telemetry.gameTime`). */
  deliveryTime: number
  plannedDistanceKm: number
  income: number
}

/** What a keybind does: the music's volume a step up or down, or it paused or played on. */
export type KeybindAction = 'volumeUp' | 'volumeDown' | 'playPause'

/** A vehicle screen's fit (cabin/state/screenFit.ts): each edge's margin, the proportion and the size. */
export interface ScreenFitValues {
  top: number
  bottom: number
  left: number
  right: number
  stretch: number
  size: number
}

export type HostMessage =
  | { type: 'telemetry'; data: Telemetry }
  | { type: 'truck'; data: TruckInfo }
  | { type: 'job'; data: JobInfo }
  /** The player's account (crates/host wallet.rs), for the phone's Nupay: its balance (`null`: no save read yet)
   * and its entries, the newest first. */
  | {
      type: 'wallet'
      balance: number | null
      currency?: GameCurrency
      liveBank?: boolean
      bankUnlocked?: boolean
      loanLimit?: number
      availableCredit?: number
      minAfterRepay?: number
      entries: WalletEntry[]
      loans?: WalletLoan[]
      offers?: WalletLoanOffer[]
      pendingLoan?: PendingLoan | null
    }
  /** Live in-memory Nupay bank RPC result. */
  | { type: 'bankAnswer'; id: number; action: string; answer: BankAnswer }
  /** Live in-memory Fleet / Garage snapshot. */
  | { type: 'fleetSnapshot'; id: number; answer: FleetAnswer; data?: FleetSnapshot }
  /** Live in-memory Fleet / Garage action result (includes refreshed snapshot on `ok`). */
  | { type: 'fleetAnswer'; id: number; action: string; answer: FleetAnswer; data?: FleetSnapshot }
  /** `null` until the game has drawn a WestPlay screen. Checked against the known ids on arrival. */
  | { type: 'screen'; layout: string | null }
  /** Adjust Screen from a phone (cabin/state/screenFit.ts), relayed by the host to the cab's page: a change, the fit
   * set back (`reset`), or neither (the phone asking what it is). */
  | { type: 'screenFit'; change?: Partial<ScreenFitValues>; reset?: boolean }
  /** The cab's page's fit on the vehicle's screen now (`key`: `null`, no screen yet), relayed to the phones. */
  | { type: 'screenFitShown'; key: string | null; fit: ScreenFitValues }
  /** The settings the page applies: whether the truck's own strip stays beside WestPlay
   * (`westplay_host::settings::TruckStrip`), the wallpaper (`null`: none chosen yet) and the player's icon set
   * (`null`: WestPlay's own). Sent as the page loads and whenever they change. */
  | {
      type: 'settings'
      truckStrip: string
      /** The split screen (`westplay_host::settings::ScreenSettings::split`): `off`, `mapLeft` or `mapRight`. */
      split?: string
      /** Live screen sharpness tier (`gameDefault`, `standard`, `high`, `ultra`). */
      screenResolution?: ScreenResolution
      /** Keep the in-cabin screen powered on even when the truck's electrics/engine are off. */
      keepScreenOn?: boolean
      wallpaper?: string | null
      iconSet?: string | null
      /** Light or dark icons, or as the screen is (`IconStyle`). */
      iconStyle?: string
      /** The player's own tone curve (Settings › Equalizer), and whether it plays (Custom) or not (Recommended). */
      equalizer?: Equalizer
      equalizerCustom?: boolean
      /** Who walks in the Modo Personagem (a place in `CHARACTER_PEOPLE`), for the phone's Personagem app. */
      characterPerson?: number
      /** The Modo Personagem's switches (the app's settings): the cargo's papers and coupling by hand, refuelling
       * by hand, paying that fuel at the pump's attendant. */
      freightPapers?: boolean
      manualRefuel?: boolean
      payAttendant?: boolean
      /** Which of the game's virtual mirrors show (the phones' Settings). */
      virtualMirrors?: VirtualMirrors
      /** Whether the game sets the suspension back by itself (the phone's Suspensão app). */
      suspensionAutoReset?: boolean
      /** The suspension by itself: lowered on the motorway, normal in town (the Suspensão app). */
      suspensionAuto?: boolean
      /** Seconds of rough road before automatic suspension raises (1..=5). */
      suspensionAutoDelay?: number
      /** Seconds of smooth road before automatic suspension lowers back down (3..=15). */
      suspensionAutoReturn?: number
      /** Seconds of hard braking before automatic suspension raises a little (1..=5). */
      suspensionAutoBrake?: number
      /** Brake fade (the phone's Freios app, owner 2026-11-21). */
      brakeFade?: boolean
      brakeFadeLevel?: BrakeFadeLevel
      brakeSquealVolume?: number
      brakeWarningVolume?: number
      brakeHud?: boolean
      brakeHudCorner?: BrakeHudCorner
      brakeHudSize?: BrakeHudSize
      brakeHudOnFoot?: boolean
      brakeSmoke?: boolean
      brakeAlarm?: number
      /** How low the front may go (the Suspensão app). */
      suspensionLowFront?: LowFront
      /** How firm the dampers are (the Suspensão app). */
      suspensionDamping?: Damping
      /** How big the mouse's dot is (the phones' Settings). */
      cursorSize?: CursorSize
      /** Where YouTube & music audio plays (`cabin` = cab speakers + muffled outside, `device` = phone/iPad). */
      audioOutput?: AudioOutput
    }
  /** The player's own wallpapers and icon sets (shared/customize.ts), their files at `CABIN_FILES`. */
  | { type: 'customize'; wallpapers: CustomWallpaper[]; iconSets: IconSet[] }
  /** The music's spectrum as it plays, while the page asks for it (`spectrum`): bars from the lows to the
   * highs, spaced by octaves, each 0 (silent) to 255; all 0 when it stops. */
  | { type: 'spectrum'; bars: number[] }
  /** Whether the host draws the screen's cursor (into the frames the game takes, where the mouse is at once), or
   * the page draws its own (crates/host cursor.rs). */
  | { type: 'cursor'; host: boolean }
  /** The cities whose garages the player owns, and their headquarters', from their latest save. */
  | { type: 'garages'; owned: string[]; headquarters: string | null }
  /** The game's answer to the page's `route` request `id`. */
  | { type: 'routeAnswer'; id: number; answer: RouteAnswer }
  /** Live in-memory Job Market snapshot (Bolsa de Fretes) for `jobMarketRefresh` request `id`. */
  | { type: 'jobMarketSnapshot'; id: number; answer: JobMarketAnswer; data?: JobMarketSnapshot }
  /** Live in-memory Job Market accept result for `jobMarketAccept` request `id`. */
  | { type: 'jobMarketAnswer'; id: number; answer: JobMarketAnswer }
  /** On-demand cargo icon PNG data URL for `jobMarketIcon` request `id`. */
  | { type: 'jobMarketIcon'; id: number; dataUrl: string | null }
  /** WestPlay's own routes for the page's `routes` request `id` (none: no road graph, or no way there). */
  | { type: 'routes'; id: number; routes: PlannedRoute[] }
  /** Whether the game's GPS follows the route picked (`follow` request `id`): `answer` "ownWay" when the
   * game takes another way however held, else the game's answer. */
  | { type: 'followed'; id: number; follows: boolean; answer: string }
  /** What stopping at each place of the page's `detours` request `id` adds to the trip (seconds on the map's
   * ground, in the order asked; `null`: not known). */
  | { type: 'detours'; id: number; seconds: (number | null)[] }
  /** What the player typed on their own keyboard, oldest first (a paste comes as its text). */
  | { type: 'typed'; events: TypedEvent[] }
  /** Whether the player's own keyboard types into the field selected. */
  | { type: 'typingState'; state: TypingState }
  /** Whether the player plays on a map by HVT, which WestPlay doesn't allow; and the game's language. */
  | { type: 'mapBlock'; hvt: boolean; language: string | null }
  /** Whether the player's profile has the game's time zones on (its clock the time where the truck is). */
  | { type: 'timeZones'; on: boolean }
  /** The game's GPS route: its map nodes in order, `[x, z, forward x, forward z, distance left]` (metres;
   * the road's direction, either way along it; the distance left from the node to the destination, as the
   * game counts distances). `null` without a route. */
  | { type: 'route'; id: number | null; nodes: [number, number, number, number, number][] | null }
  /** The ways through the route's junctions (the junction's own lanes, from the game's files): each from
   * the node it starts at to the next, as `[x, z]` points; and the turns there, by the same node: the
   * turn (`right`, `exit-left`, `roundabout`…), its angle (degrees, right positive) and a roundabout's
   * exit; and how far to the right of its way the route runs at its nodes on roads (metres: on the
   * carriageway it drives on, where a road is drawn as two), by node. */
  | {
      type: 'routePaths'
      id: number
      paths: [number, [number, number][]][]
      maneuvers: [number, string, number, number | null][]
      shifts: [number, number][]
    }
  /** The truck's progress along the route: the index of the map node ahead of it. */
  | { type: 'routeNext'; next: number }
  /** Open (or close) WestPlay on the truck's screen: the companion window's switch. */
  | { type: 'westplay'; open: boolean }
  /** The game's weather (`null` when it can't be read: no game, or a game version whose weather the host
   * doesn't know). */
  | { type: 'weather'; weather: GameWeather | null }
  /** The brakes (host brake_fade.rs): the truck's and the trailer's discs (°C; the trailer's `null` without
   * one) and how much brake is left (percent). Once a second. */
  | { type: 'brakes'; truck: number; trailer: number | null; strength: number }
  /** How YouTube's player plays (`null`: YouTube isn't running). */
  | { type: 'youtube'; state: YouTubeState | null }
  /** YouTube's answer to the page's question `id` (a `youtube` command with an `id`). */
  | ({ type: 'youtubeAnswer'; id: number } & YouTubeAnswer)
  /** Whether the game's window shows (`false`: minimised, or on another desktop; in front or not: Lossless
   * Scaling's window is in front of it). */
  | { type: 'gameShown'; shown: boolean }
  /** Whether WestPlay's screen is in the game's view (not with the camera outside the cab, nor in its menu). */
  | { type: 'screenSeen'; seen: boolean }
  /** The player's stops along the route `id`: their map nodes' indices in it, ascending (not WestPlay's own
   * waypoints, nor the destination). */
  | { type: 'routeStops'; id: number | null; stops: number[] }
  /** The game's menu gone while it's still paused: its still picture before the truck moves on. */
  | { type: 'menuGone'; gone: boolean }
  /** The phone in the game taken out on one of its apps (the keybind for its Personagem: crates/host). */
  | { type: 'openApp'; app: string }
  /** TikTok under the phone in the game's page (crates/host tiktok.rs): whether it's signed in (`null`: not known
   * yet) and as whom, whether its sign-in window is open, and whether the account's LIVE is on. */
  | { type: 'tiktok'; signedIn: boolean | null; user: string | null; signingIn: boolean; live: boolean }
  /** A notification for the phone to show and sound: what happened on the player's TikTok LIVE (who followed,
   * sent a gift, shared or subscribed; a gift's name and how many). */
  | ({ type: 'phoneNotification' } & PhoneNotification)
  /** The cargo's paper for the phone in the game's page to show in place of the phone (crates/host; the plugin's
   * `freight`): the nota fiscal to sign at the pick-up, the recipient's signature at the delivery; `null`: none. */
  | { type: 'paper'; stage: 'pickup' | 'delivery' | 'lockPull' | 'lockPush' | 'legsUp' | 'legsDown' | null }
  /** Whether the suspension's auto reset is in the games' options as chosen (else: once the games close). */
  | { type: 'suspensionOption'; applied: boolean }
  /** The fuel filled by hand at a pump, to pay on the phone's Nupay at its attendant (crates/host; the plugin's
   * `refuel`); `null`: none. */
  | { type: 'fuelBill'; bill: FuelBill | null }
  /** A keybind of the player's (the companion's Controls), pressed in the game. */
  | { type: 'keybind'; action: KeybindAction }
  /** A volume the player's phone set (0 to 1), for the cab to show as for a keybind (crates/host remote.rs). */
  | { type: 'volumeShown'; volume: number }
  /** Whether the game runs. As it closes, the host closes the web players; the page powers off and forgets
   * its last telemetry. */
  | { type: 'game'; running: boolean }
  /** Whether the game runs under TruckersMP (the page shows its other players, from TruckersMP's map feed). */
  | { type: 'truckersmp'; on: boolean }
  /** Under TruckersMP, the players around the player from its SDK (the plugin's): its game server's id, the
   * player's own number on it, and each other player's number, world x and z (m) and heading (turns); `null`s
   * and none when there are none to tell. */
  | { type: 'nearby'; server: number | null; local: number | null; players: [number, number, number, number][] }
  /** Whether YouTube is signed in, and the picture to show for the player: nothing else of the account. */
  | { type: 'youtubeAccount'; signedIn: boolean; avatar: string | null }
  /** Whether Google's sign-in window is open on the PC's desktop. */
  | { type: 'youtubeSignIn'; open: boolean }
  /** Whether YouTube's video visual is currently shown inside the in-game iPad/phone (not under the cab). */
  | { type: 'youtubeHandset'; on: boolean }
  /** The map the cab shows: its folder in WestPlay's maps (served at `https://westplay.maps/`), and when it
   * was built; `null` for none (none built yet). */
  | { type: 'map'; folder: string | null; built: number | null }
  /** Spotify's web player: whether it's signed in and playing (`null`: not running), and whether its sign-in
   * window is open on the PC's desktop. Nothing else of the account. */
  | { type: 'spotify'; signedIn: boolean | null; playing: boolean | null; signingIn: boolean }
  /** Spotify's answer to the Spotify app's question `id` (`failed`: Spotify didn't answer). */
  | ({ type: 'spotifyAnswer'; id: number; failed?: boolean } & Partial<SpotifyAnswers>)
  /** What Spotify plays (on WestPlay, or elsewhere on the account through Spotify Connect); `null`: nothing. */
  | { type: 'spotifyPlayback'; playback: SpotifyPlayback | null }
  /** Apple Music's web player: whether it's signed in and playing (`null`: not running), and whether its
   * sign-in window is open on the PC. */
  | {
      type: 'appleMusic'
      signedIn: boolean | null
      playing: boolean | null
      signingIn: boolean
      /** The account's picture (its Apple Music profile's), if it has one. */
      avatar: string | null
    }
  /** Apple Music's answer to the Music app's question `id` (`failed`: Apple Music didn't answer). */
  | ({ type: 'appleMusicAnswer'; id: number; failed?: boolean } & Partial<AppleMusicAnswers>)
  /** What Apple Music plays; `null`: nothing. */
  | { type: 'appleMusicPlayback'; playback: AppleMusicPlayback | null }
  /** YouTube Music's web player: whether it's signed in and playing (`null`: not running), whether its sign-in
   * window is open on the PC, and the account's picture (Google's). YouTube's sign-in: they share it. */
  | {
      type: 'youtubeMusic'
      signedIn: boolean | null
      playing: boolean | null
      signingIn: boolean
      avatar: string | null
    }
  /** YouTube Music's answer to the app's question `id` (`failed`: YouTube Music didn't answer). */
  | ({ type: 'youtubeMusicAnswer'; id: number; failed?: boolean } & Partial<YouTubeMusicAnswers>)
  /** What YouTube Music plays; `null`: nothing. */
  | { type: 'youtubeMusicPlayback'; playback: YouTubeMusicPlayback | null }

/** The game's weather: the one there is and the one it's turning to (the same once it has); in game minutes,
 * how long until the game changes it next; and how wet the roads are, percent. Each of the last two `null`
 * (or missing, from an older host) when the host can't read it. */
export interface GameWeather {
  now: Sky
  coming: Sky
  changeIn?: number | null
  wetness?: number | null
}

/** One of the game's two weathers. */
export type Sky = 'clear' | 'rain'

/** Messages from the cabin page to the host. */
/** The players' tone controls, as a car stereo's: bass, mid and treble, in dB each (`EQUALIZER_MOST` either way;
 * the host's `speakers::Equalizer`). */
export interface Equalizer {
  bass: number
  mid: number
  treble: number
}

export const EQUALIZER_BANDS = ['bass', 'mid', 'treble'] as const satisfies readonly (keyof Equalizer)[]

/** How far each band goes, dB (the host's `Equalizer::MOST`). */
export const EQUALIZER_MOST = 6

export const FLAT: Equalizer = { bass: 0, mid: 0, treble: 0 }

/** The tone as the host may send it: each band a whole number of dB within the range (anything else flat). */
export function toEqualizer(value: unknown): Equalizer {
  const told = (value ?? {}) as Partial<Record<keyof Equalizer, unknown>>
  const band = (db: unknown) =>
    typeof db === 'number' && Number.isFinite(db)
      ? Math.max(-EQUALIZER_MOST, Math.min(EQUALIZER_MOST, Math.round(db)))
      : 0
  return { bass: band(told.bass), mid: band(told.mid), treble: band(told.treble) }
}

export type PageMessage =
  | { type: 'ready' }
  /** Adjust Screen from a phone (cabin/state/screenFit.ts), relayed by the host to the cab's page: a change, the fit
   * set back (`reset`), or neither (the phone asking what it is). */
  | { type: 'screenFit'; change?: Partial<ScreenFitValues>; reset?: boolean }
  /** The cab's page's fit on the vehicle's screen now (`key`: `null`, no screen yet), relayed to the phones. */
  | { type: 'screenFitShown'; key: string | null; fit: ScreenFitValues }
  /** A startup stage reached (the boot screens shown), for the host's startup log. */
  | { type: 'startup'; stage: string }
  /** Whether WestPlay is open on the truck's screen (or the truck's original screen shows): posted as the
   * page connects and whenever it changes, for the companion window. */
  | { type: 'westplay'; open: boolean }
  /** The wallpaper chosen in Settings (wallpapers.ts), kept with WestPlay's settings (the companion's too). */
  | { type: 'wallpaper'; id: string }
  /** The Vehicle Bar (the truck's strip) kept or not, chosen in Settings › Appearance, kept the same way. */
  | { type: 'truckStrip'; strip: 'auto' | 'off' }
  /** Live screen sharpness tier chosen in Settings › Adjust Screen, applied immediately on the next frame. */
  | { type: 'screenResolution'; resolution: ScreenResolution }
  /** Keep the in-cabin screen powered on even when the truck's electrics/engine are off. */
  | { type: 'keepScreenOn'; on: boolean }
  /** The players' tone, set in Settings › Equalizer: the player's curve, and whether it plays (Custom) or the
   * speakers as tuned (Recommended); kept with WestPlay's settings and played by every player. */
  | ({ type: 'equalizer'; custom: boolean } & Equalizer)
  /** The music's spectrum wanted (Settings › Equalizer open) or not: the host answers `spectrum` as it plays. */
  | { type: 'spectrum'; on: boolean }
  /** Whether what's under the cursor is light: the host draws it dark there. */
  | { type: 'cursorLight'; light: boolean }
  /** A volume the phone page set (0 to 1): the host has the cab show it (crates/host remote.rs). */
  | { type: 'volumeShown'; volume: number }
  /** One of the truck's commands (the game's control by name: `westplay_shared::TRUCK_COMMANDS`) pressed or let
   * go in the phone's Commands app; the plugin gives it to the game as held (crates/host truck_commands.rs). */
  | { type: 'truckCommand'; command: string; down: boolean }
  /** A change to the game's GPS route (Maps' search), at a map node (its uid, hex) for going or adding a
   * stop; the host answers with `routeAnswer` and the same `id`. */
  | { type: 'route'; id: number; action: RouteAction; node?: string }
  /** The routes to a map node (its uid, hex) from where the truck is, to offer before Go (Directions). */
  | { type: 'routes'; id: number; node: string }
  /** Go along the route at `index` of those offered for `routes` request `id`: the host hands it to the
   * game's GPS and answers `followed`. */
  | { type: 'follow'; id: number; index: number }
  /** What stopping at each of these map nodes (uids, hex) would add to the trip the game's route makes. */
  | { type: 'detours'; id: number; nodes: string[] }
  /** Nupay's history let go, its balance kept (crates/host wallet.rs): the account comes back without it. */
  | { type: 'walletClear' }
  /** Request or cancel a new bank loan queued for the next game restart (owner 2026-11-22). */
  | { type: 'walletLoan'; amount: number; days: number; rate: number }
  | { type: 'walletLoanCancel' }
  /** Who walks in the Modo Personagem, chosen in the phone's Personagem app (a place in `CHARACTER_PEOPLE`). */
  | { type: 'characterPerson'; person: number }
  /** The text field selected, a number that changes with every selection (0: none), for the player's own
   * keyboard to type into while the truck stands still. */
  | { type: 'typing'; selection: number }
  /** A command for YouTube's player, or a question for YouTube (answered with `youtubeAnswer`). */
  | { type: 'youtube'; command: YouTubeCommand }
  /** A command for Spotify's web player (`westplay_host::spotify::Command`): `show` places it, in the frame's
   * pixels, where the Spotify app cuts the page open. */
  | { type: 'spotify'; command: SpotifyCommand }
  /** A command for Apple Music's web player (`westplay_host::apple_music::Command`). */
  | { type: 'appleMusic'; command: AppleMusicCommand }
  /** A command for YouTube Music's web player (`westplay_host::youtube_music::Command`). */
  | { type: 'youtubeMusic'; command: YouTubeMusicCommand }
  /** A command for TikTok under the phone in the game's page (`westplay_host::tiktok::Command`): the phone in
   * the game's alone to send. */
  | { type: 'tiktok'; command: TikTokCommand }
  /** A command for YouTube running in the in-game iPad/phone (shares YouTube's ad-free engine). */
  | { type: 'handsetYoutube'; command: YouTubeCommand }
  /** Whether a notification is on the phone's screen: put away, its top stays in view meanwhile. */
  | { type: 'phoneBanner'; showing: boolean }
  /** The game's notifications' groups turned off in the phone's Settings. */
  | { type: 'alerts'; off: AlertGroup[] }
  /** One of each of the game's notifications, to see and hear them. */
  | { type: 'testAlerts' }
  /** The cargo's paper collected, signed and stamped, or put down (`collected` false): the phone in the game's. */
  | { type: 'paperDone'; collected: boolean }
  /** The fuel's bill paid on Nupay: `amount` in the game's money (negative), about `detail`. */
  | { type: 'fuelPaid'; id: number; amount: number; detail: string }
  /** The fuel's bill put down without paying. */
  | { type: 'fuelPutDown'; id: number }
  /** The cargo's papers and coupling the trailer by hand, on or off (the phone's Settings). */
  | { type: 'freightPapers'; on: boolean }
  /** Refuelling by hand at the truck's tank, on or off (the phone's Settings). */
  | { type: 'manualRefuel'; on: boolean }
  /** Paying the fuel filled by hand at the pump's attendant, on or off (a phone's Settings). */
  | { type: 'payAttendant'; on: boolean }
  /** Which of the game's virtual mirrors show on the screen (a phone's Settings, owner 2026-11-18). */
  | { type: 'virtualMirrors'; which: VirtualMirrors }
  /** Whether the game sets the suspension back by itself (the phone's Suspensão app). */
  | { type: 'suspensionAutoReset'; on: boolean }
  /** The suspension by itself, on or off (the Suspensão app, owner 2026-11-19). */
  | { type: 'suspensionAuto'; on: boolean }
  /** Seconds of rough road before automatic suspension raises (1..=5, owner 2026-11-22). */
  | { type: 'suspensionAutoDelay'; secs: number }
  /** Seconds of smooth road before automatic suspension lowers back down (3..=15, owner 2026-11-22). */
  | { type: 'suspensionAutoReturn'; secs: number }
  /** Seconds of hard braking before automatic suspension raises a little (1..=5, owner 2026-11-23). */
  | { type: 'suspensionAutoBrake'; secs: number }
  /** Brake fade (the Freios app, owner 2026-11-21): on or off, how soon, the sounds' volumes (0 off..100), the HUD. */
  | { type: 'brakeFade'; on: boolean }
  | { type: 'brakeFadeLevel'; which: BrakeFadeLevel }
  | { type: 'brakeSqueal'; volume: number }
  | { type: 'brakeWarning'; volume: number }
  | { type: 'brakeHud'; on: boolean }
  | { type: 'brakeHudCorner'; which: BrakeHudCorner }
  | { type: 'brakeHudSize'; which: BrakeHudSize }
  | { type: 'brakeHudOnFoot'; on: boolean }
  | { type: 'brakeSmoke'; on: boolean }
  /** When the brake alarm comes (°C on the realistic level). */
  | { type: 'brakeAlarm'; celsius: number }
  /** How low the front may go (the Suspensão app): counts from the game's next start. */
  | { type: 'suspensionLowFront'; which: LowFront }
  /** How firm the dampers are (the Suspensão app): counts from the game's next start. */
  | { type: 'suspensionDamping'; which: Damping }
  /** How big the mouse's dot is on the truck's screen and the phone (a phone's Settings, owner 2026-11-19). */
  | { type: 'cursorSize'; which: CursorSize }
  /** Where the reversing camera shows the game's mirrors (frame pixels, the page see-through there); width 0: not. */
  | { type: 'mirrorCamera'; x: number; y: number; width: number; height: number }
  /** The reversing camera moved by the player on its screen (crates/plugin rear_camera.rs `nudged`), kept for the
   * truck or trailer it's on. */
  | { type: 'cameraNudge'; action: CameraNudge }
  /** MaxDevMod's floating screen in the cab (crates/plugin float_screen.rs): switched on or off, moved, turned. */
  | { type: 'floatScreen'; action: FloatScreenAction }
  /** Live in-memory Job Market (Bolsa de Fretes): ask the plugin to snapshot live offers. */
  | { type: 'jobMarketRefresh'; id: number }
  /** Live in-memory Job Market: ask the plugin to accept an offer directly on the main thread. */
  | { type: 'jobMarketAccept'; id: number; mode: JobMarketMode; companyPtr: string; offerPtr: string }
  /** Live in-memory Job Market: ask the plugin to cancel the active job on the main thread. */
  | { type: 'jobMarketCancel'; id: number }
  /** On-demand cargo icon image lookup from game/DLC/mod archives. */
  | { type: 'jobMarketIcon'; id: number; icon: string; cargoId: string }
  /** Live in-memory Nupay Bank RPCs (zero restart). */
  | { type: 'bankRefresh'; id: number }
  | { type: 'bankTakeLoan'; id: number; offerIndex: number }
  | { type: 'bankRepayLoan'; id: number; loanIndex: number }
  /** Live in-memory Company / Fleet & Garage RPCs (zero restart). */
  | { type: 'fleetRefresh'; id: number }
  | { type: 'fleetDriveTruck'; id: number; truckPtr: string }
  | { type: 'fleetUseTrailer'; id: number; trailerPtr: string }
  | { type: 'fleetStoreTrailer'; id: number }
  | { type: 'fleetUpgradeGarage'; id: number; garagePtr: string }
  /** Switch the in-game handheld overlay live between iPhone (340x720) and vertical iPad (560x760). */
  | { type: 'phoneDevice'; device: 'phone' | 'ipad' }
  /** Switch media audio output live between truck cab speakers (`cabin`) and phone/iPad (`device`). */
  | { type: 'audioOutput'; which: AudioOutput }

/** What the phone's TikTok app asks of TikTok's own site, run by the host under the phone's page
 * (`westplay_host::tiktok::Command`): `show` places it, in the phone's pixels, where the app cuts the page open. */
export type TikTokCommand =
  | { action: 'start' }
  | { action: 'show'; x: number; y: number; width: number; height: number }
  | { action: 'hide' }
  | { action: 'go'; page: TikTokPage }
  | { action: 'back' }
  /** Its videos' volume a step up (1) or down (-1): the phone's side keys. */
  | { action: 'volume'; step: 1 | -1 }
  /** Whether the account's LIVE is watched for the phone's notifications. */
  | { action: 'notifications'; on: boolean }
  /** A made-up follow, to see and hear a notification. */
  | { action: 'testNotification' }

export type FloatScreenAction =
  | 'toggle'
  | 'forward'
  | 'back'
  | 'up'
  | 'down'
  | 'left'
  | 'right'
  | 'turnLeft'
  | 'turnRight'
  | 'tiltUp'
  | 'tiltDown'
  | 'rollLeft'
  | 'rollRight'
  | 'reset'
  | 'bigger'
  | 'smaller'

export type CameraNudge =
  | 'back'
  | 'forward'
  | 'backFar'
  | 'forwardFar'
  | 'up'
  | 'down'
  | 'left'
  | 'right'
  | 'reset'
  | 'tiltUp'
  | 'tiltDown'
  | 'turnLeft'
  | 'turnRight'
  | 'rollLeft'
  | 'rollRight'
  | 'wider'
  | 'narrower'

export type TikTokPage = 'forYou' | 'following' | 'explore' | 'live' | 'profile'

/** A notification on the phone: TikTok's (`westplay_host::tiktok::Event`), or the game's and MaxDevMod's
 * (`westplay_host::alerts::Alert`: money in and out, the truck, the job). */
export type PhoneNotification = TikTokNotification | GameNotification

export interface TikTokNotification {
  app: 'tiktok'
  kind: 'follow' | 'gift' | 'share' | 'subscribe'
  /** Who did it, as TikTok shows them. */
  user: string
  /** A gift's name, and how many of it. */
  detail?: string
  count?: number
}

export interface GameNotification {
  /** The app it's of: its icon and name on the banner. */
  app: 'nupay' | 'truck' | 'freight'
  kind: GameNotificationKind
  /** A sum in the game's money (`currency`; an expense negative). */
  amount?: number
  currency?: GameCurrency
  /** A number: the damage's percent, minutes, the fuel's range in km. */
  value?: number
  /** About what: the cargo and where to, the offence, the truck's part. */
  detail?: string
  /** How many of the same, told as one. */
  count?: number
  /** A sum in the pump's own money, cents and all (its sign in `detail`): the fuel owed to the attendant. */
  total?: number
}

export type GameNotificationKind =
  | 'payAttendant'
  | 'delivered'
  | 'cancelled'
  | 'fined'
  | 'toll'
  | 'ferry'
  | 'train'
  | 'fuel'
  | 'otherIn'
  | 'otherOut'
  | 'fuelLow'
  | 'damage'
  | 'crash'
  | 'repaired'
  | 'rest'
  | 'restNow'
  | 'jobNew'
  | 'jobDeadline'
  | 'jobLate'
  | 'brakeHot'
  | 'brakeGone'

/** The groups of the game's notifications the phone's Settings turn on and off (`westplay_host::alerts::Group`). */
export const ALERT_GROUPS = ['moneyIn', 'moneyOut', 'fine', 'fuel', 'damage', 'brake', 'rest', 'job'] as const
export type AlertGroup = (typeof ALERT_GROUPS)[number]

/** What the Spotify app asks of Spotify's web player (`westplay_host::spotify::Command`), run hidden by the
 * host: its sign-in, a question (answered with `spotifyAnswer` and the same `id`), and the player's controls
 * (`play` with a context plays it, from `track`; without, it plays on). */
export type SpotifyCommand =
  /** Run (hidden), saying whether it's signed in (the Spotify app opened). */
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signUp' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: SpotifyQuestion }
  | { action: 'play'; context?: string; track?: string }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  /** The playing device's volume (Spotify's own), 0 to 100. */
  | { action: 'volume'; percent: number }
  /** The cab's fade, in the page's own player: its volume times `level` (0 to 1; 1, as Spotify has it). */
  | { action: 'fade'; level: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: SpotifyRepeat }
  /** Into Liked Songs or Your Library (`on`), or out of it. */
  | { action: 'save'; uri: string; on: boolean }

export type SpotifyRepeat = 'off' | 'all' | 'one'

/** What a row, a card or a shortcut shows: under its name, a card shows its description (or who it's by), a
 * row what it is and who it's by ("Playlist • Spotify", built in the cab's language). A song as a card (Home's
 * Trending songs) plays from its album (`context`). */
export interface SpotifyItem {
  uri: string
  kind: 'playlist' | 'album' | 'artist' | 'show' | 'liked' | 'track'
  name: string
  /** The playlist's owner, the album's artists, the podcast's publisher, the song's artists ("" for an
   * artist). */
  by: string
  description: string
  /** Liked Songs' count (its row says it in place of an owner). */
  songs?: number
  image: string | null
  /** The cover's own colour (Spotify extracts one for every image): pages and the player are washed in it. */
  color: string
  pinned?: boolean
  downloaded?: boolean
  /** In Your Library (Spotify says, signed in). */
  saved?: boolean
  /** A song's album. */
  context?: string
}

export interface SpotifyArtistName {
  uri: string
  name: string
}

export interface SpotifyTrack {
  uri: string
  name: string
  artists: SpotifyArtistName[]
  album: { uri: string; name: string }
  image: string | null
  color: string
  /** Seconds. */
  duration: number
  explicit: boolean
  /** In Liked Songs (Spotify says, signed in). */
  saved?: boolean
}

/** A playlist's or album's page. */
export interface SpotifyCollection {
  item: SpotifyItem
  description: string
  /** "Spotify", or the album's artist; and an album's year. */
  owner: string
  year: number | null
  tracks: SpotifyTrack[]
  /** Where its songs go on, while there are more than these: asked as the question's `more`. */
  more?: string
}

export interface SpotifyArtist {
  item: SpotifyItem
  listeners: number
  verified: boolean
  popular: SpotifyTrack[]
  releases: SpotifyItem[]
}

/** Home: Spotify's chips (each a facet of Home, its own words), its shortcuts, and its shelves. */
export interface SpotifyHome {
  chips: { id: string; label: string }[]
  shortcuts: SpotifyItem[]
  /** Its shelves; `recent`: Spotify's Recently played. */
  sections: { title: string; items: SpotifyItem[]; recent?: boolean }[]
}

export interface SpotifyCategory {
  name: string
  color: string
  /** A cover leaning in its corner, as Spotify's tiles have. */
  cover: SpotifyItem | null
}

export interface SpotifyResults {
  top: SpotifyItem | null
  /** The top result when it's a song. */
  topTrack: SpotifyTrack | null
  tracks: SpotifyTrack[]
  artists: SpotifyItem[]
  playlists: SpotifyItem[]
  albums: SpotifyItem[]
}

/** What the Spotify app asks Spotify: Home (a facet of it, `''` for all; the player's own signed in,
 * Spotify's public one otherwise), Your Library, Browse all, a search, a playlist's or album's page, an
 * artist's. */
export type SpotifyQuestion =
  | { ask: 'home'; facet: string; signedIn: boolean }
  | { ask: 'lyrics'; uri: string }
  | { ask: 'library' }
  | { ask: 'browse' }
  | { ask: 'search'; query: string }
  | { ask: 'collection'; uri: string; more?: string }
  | { ask: 'artist'; uri: string }

/** Each question's answer, by the question. */
export interface SpotifyAnswers {
  home: SpotifyHome
  library: SpotifyItem[]
  browse: SpotifyCategory[]
  search: SpotifyResults
  collection: SpotifyCollection
  artist: SpotifyArtist
  lyrics: Lyrics
}

/** What Spotify plays: the song, where it plays from, how far (seconds, as the host said it), the options,
 * and the songs to come. */
export interface SpotifyPlayback {
  track: SpotifyTrack | null
  context: { uri: string; name: string; kind: SpotifyItem['kind'] } | null
  playing: boolean
  position: number
  duration: number
  shuffle: boolean
  repeat: SpotifyRepeat
  /** The playing device's volume, 0 to 1 (`null`: none Spotify can set). */
  volume: number | null
  /** Playing on WestPlay's own device, not another of the account's (a phone); missing: taken as here. */
  here?: boolean
  next: SpotifyTrack[]
}

/** What the Music app asks of Apple Music's web player (`westplay_host::apple_music::Command`), run hidden by
 * the host: its sign-in (MusicKit's own window, on the PC), a question (answered with `appleMusicAnswer` and
 * the same `id`), and the player's controls. Ids are Apple Music's type and its own id: `albums:1440…`. */
export type AppleMusicCommand =
  /** Run (hidden), saying whether it's signed in (the Music app opened). */
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: AppleMusicQuestion }
  /** An album, playlist, station, artist or song (`id`), or a list of songs (`songs`), from `index`; with
   * neither, play on. */
  | { action: 'play'; id?: string; songs?: string[]; index?: number; shuffle?: boolean }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: AppleMusicRepeat }
  /** A song of the queue, `offset` songs on. */
  | { action: 'jump'; offset: number }
  | { action: 'favorite'; id: string; on: boolean }
  | { action: 'add'; id: string }

export type AppleMusicRepeat = 'off' | 'all' | 'one'
export type AppleMusicShelf = 'recent' | 'playlists' | 'artists' | 'albums' | 'songs'

/** A card or row: what it is, its name, who it's by (an album's artist, a playlist's curator, a station's
 * tagline), Apple's editors' note, its cover (and a wide picture, Apple's editors' own) and the cover's
 * colour. */
export interface AppleMusicItem {
  id: string
  /** A folder: of the Library's playlists. */
  kind: 'album' | 'playlist' | 'artist' | 'station' | 'song' | 'curator' | 'folder'
  name: string
  by: string
  note: string
  image: string | null
  wide: string | null
  color: string
  explicit: boolean
  /** A live station (Apple Music 1…). */
  live: boolean
  /** A big card's label ("UPDATED PLAYLIST"). */
  label?: string
  /** A favourite (the Library's lists say). */
  favorite?: boolean
}

export interface AppleMusicTrack {
  id: string
  name: string
  artist: string
  album: string
  albumId: string | null
  artistId: string | null
  image: string | null
  color: string
  /** Seconds. */
  duration: number
  explicit: boolean
  /** Its number on its album. */
  number: number | null
  /** A favourite (Apple Music says, signed in, for the song playing). */
  favorite?: boolean
}

export interface AppleMusicSection {
  title: string
  /** Big cards with the editors' pictures, a shelf of covers, what was played lately, or songs. */
  style: 'hero' | 'shelf' | 'recent' | 'songs'
  items: AppleMusicItem[]
  songs: AppleMusicTrack[]
}

/** Home, New, Radio or a category's page. */
export interface AppleMusicPage {
  sections: AppleMusicSection[]
}

export interface AppleMusicCollection {
  item: AppleMusicItem
  description: string
  genre: string
  year: number | null
  tracks: AppleMusicTrack[]
}

export interface AppleMusicArtist {
  item: AppleMusicItem
  top: AppleMusicTrack[]
  sections: AppleMusicSection[]
}

export interface AppleMusicResults {
  top: AppleMusicItem[]
  songs: AppleMusicTrack[]
  artists: AppleMusicItem[]
  albums: AppleMusicItem[]
  playlists: AppleMusicItem[]
  stations: AppleMusicItem[]
}

export interface AppleMusicLibrary {
  items: AppleMusicItem[]
  songs: AppleMusicTrack[]
  /** The sorts the list has (Apple Music's own, named in the cab's language; a playlist's unnamed: the cab
   * names them), and the one it's in. */
  sorts: { id: string; label: string }[]
  sort: string | null
}

/** What the Music app asks Apple Music: Home (signed in), New, Radio, Search's categories, a category's page,
 * a search, an album's or playlist's page, an artist's, a list of the Library. */
export type AppleMusicQuestion =
  | { ask: 'home' }
  | { ask: 'new' }
  | { ask: 'radio' }
  | { ask: 'browse' }
  | { ask: 'curator'; id: string }
  | { ask: 'search'; query: string }
  | { ask: 'collection'; id: string }
  | { ask: 'artist'; id: string }
  | { ask: 'library'; shelf: AppleMusicShelf; sort?: string; folder?: string; language?: 'en' | 'pt' | 'tr' }
  | { ask: 'lyrics'; id: string }

export interface AppleMusicAnswers {
  home: AppleMusicPage
  new: AppleMusicPage
  radio: AppleMusicPage
  browse: AppleMusicItem[]
  curator: AppleMusicPage
  search: AppleMusicResults
  collection: AppleMusicCollection
  artist: AppleMusicArtist
  library: AppleMusicLibrary
  lyrics: Lyrics
}

/** A song's words, as Apple Music and Spotify answer them (`westplay_host::lyrics::Lyrics`): line by line with
 * when each is sung (`synced`; seconds, `end` 0: until the next), where a stanza starts, where they're from
 * (Spotify's provider, Apple Music's songwriters), Spotify's colours for them, and whether they're written
 * right to left. No lines: no words for the song. */
export interface Lyrics {
  synced: boolean
  lines: { start: number; end: number; text: string; stanza?: boolean }[]
  provider?: string
  writers?: string[]
  colors?: { background: string; text: string; highlight: string }
  rtl?: boolean
}

/** What Apple Music plays: the song, where from (as the app asked for it), how far (seconds), whether it's
 * the song's 90-second preview (signed out, or no subscription), the options, the volume and what's next. */
export interface AppleMusicPlayback {
  track: AppleMusicTrack | null
  context: { id: string; name: string; kind: AppleMusicItem['kind'] } | null
  playing: boolean
  /** Playing as asked, its sound not come yet (loading, waiting for its stream, seeking): its time still. */
  loading?: boolean
  position: number
  duration: number
  preview: boolean
  shuffle: boolean
  repeat: AppleMusicRepeat
  /** The page's player's volume, 0 to 1. */
  volume: number | null
  next: AppleMusicTrack[]
}

/** What the YouTube Music app asks of YouTube Music's web player (`westplay_host::youtube_music::Command`), run
 * hidden by the host: its sign-in (Google's, YouTube's own: they share it), a question (answered with
 * `youtubeMusicAnswer` and the same `id`), the language it answers in, and the player's controls (`play` with
 * `watch` plays it; without, it plays on). */
export type YouTubeMusicCommand =
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: YouTubeMusicQuestion }
  | { action: 'language'; code: 'en' | 'pt' | 'tr' }
  | { action: 'play'; watch?: YouTubeMusicWatch }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: YouTubeMusicRepeat }
  /** The queue's song at `index` (its place in the whole queue). */
  | { action: 'jump'; index: number }
  | { action: 'like'; video: string; like: YouTubeMusicRating }
  /** A song into the queue: right after the one playing (`next`), or at its end. */
  | { action: 'queue'; video: string; next: boolean }
  /** An album or playlist (its playlist id) saved to the Library, or taken out. */
  | { action: 'save'; playlist: string; on: boolean }
  /** The Song/Video switch: the music video's picture (`true`), or the song. */
  | { action: 'video'; on: boolean }
  /** The music video's picture placed at this rectangle of the frame (pixels), under the page; or none. */
  | { action: 'show'; x: number; y: number; width: number; height: number }
  | { action: 'hide' }

export type YouTubeMusicRepeat = 'off' | 'all' | 'one'
export type YouTubeMusicRating = 'like' | 'dislike' | 'none'
export type YouTubeMusicShelf = 'playlists' | 'songs' | 'albums' | 'artists' | 'subscriptions' | 'history'
export type YouTubeMusicKind =
  'song' | 'video' | 'episode' | 'album' | 'playlist' | 'artist' | 'channel' | 'podcast' | 'mood'

/** Playing something: a song (`video`), from a list (`playlist`, at `index`), with YouTube's own `params` (a
 * radio, a shuffle), as YouTube Music's answers give them. */
export interface YouTubeMusicWatch {
  video?: string
  playlist?: string
  params?: string
  index?: number
}

/** A page of YouTube Music's: its id, and its params (a mood's, a chart's). */
export interface YouTubeMusicBrowse {
  id: string
  params?: string
}

/** A card or a row: what it is, its title and YouTube Music's own line under it, its picture, its page,
 * playing it, a song's id, artist and album pages and length, a mood's colour, its place (a number). */
export interface YouTubeMusicItem {
  kind: YouTubeMusicKind
  title: string
  subtitle: string
  image?: string
  browse?: YouTubeMusicBrowse
  play?: YouTubeMusicWatch
  video?: string
  artist?: YouTubeMusicBrowse
  album?: YouTubeMusicBrowse
  duration?: string
  explicit?: boolean
  color?: string
  rank?: string
}

/** Square covers side by side; songs in columns of four (Quick picks); songs one under another; mood
 * buttons, all of them or in rows side by side; words. */
export type YouTubeMusicStyle = 'shelf' | 'columns' | 'songs' | 'moods' | 'moodRows' | 'about'

export interface YouTubeMusicSection {
  title: string
  strapline?: string
  style: YouTubeMusicStyle
  items: YouTubeMusicItem[]
  text?: string
  more?: YouTubeMusicBrowse
}

export interface YouTubeMusicChip {
  label: string
  params?: string
  selected?: boolean
}

/** Home, Explore, a page, a song's Related, the Library's: its choices, shelves and the token for more. */
export interface YouTubeMusicPage {
  title?: string
  /** The picture YouTube Music shows behind its top (Home's, as its mood is). */
  background?: string
  chips: YouTubeMusicChip[]
  sections: YouTubeMusicSection[]
  more?: string
}

export interface YouTubeMusicCollection {
  kind: 'album' | 'playlist'
  title: string
  subtitle: string
  second: string
  by: string
  byPage?: YouTubeMusicBrowse
  image?: string
  description: string
  play?: YouTubeMusicWatch
  shuffle?: YouTubeMusicWatch
  radio?: YouTubeMusicWatch
  playlist?: string
  saved?: boolean
  tracks: YouTubeMusicItem[]
  sections: YouTubeMusicSection[]
  more?: string
}

export interface YouTubeMusicArtist {
  name: string
  description: string
  image?: string
  subscribers: string
  shuffle?: YouTubeMusicWatch
  radio?: YouTubeMusicWatch
  sections: YouTubeMusicSection[]
}

export interface YouTubeMusicResults {
  top?: YouTubeMusicItem
  topSongs: YouTubeMusicItem[]
  chips: YouTubeMusicChip[]
  items: YouTubeMusicItem[]
  more?: string
}

export interface YouTubeMusicSuggestions {
  texts: string[]
  items: YouTubeMusicItem[]
}

export interface YouTubeMusicLyrics {
  text: string
  source: string
}

export type YouTubeMusicQuestion =
  | { ask: 'home'; chip?: string; more?: string }
  | { ask: 'explore' }
  | { ask: 'browse'; page: YouTubeMusicBrowse }
  | { ask: 'search'; query: string; filter?: string; more?: string }
  | { ask: 'suggest'; query: string }
  | { ask: 'collection'; id: string; more?: string }
  | { ask: 'artist'; id: string }
  | { ask: 'library'; shelf: YouTubeMusicShelf }
  | { ask: 'lyrics'; video: string }
  | { ask: 'related'; video: string }

export interface YouTubeMusicAnswers {
  home: YouTubeMusicPage
  explore: YouTubeMusicPage
  browse: YouTubeMusicPage
  search: YouTubeMusicResults
  suggest: YouTubeMusicSuggestions
  collection: YouTubeMusicCollection
  artist: YouTubeMusicArtist
  library: YouTubeMusicPage
  lyrics: YouTubeMusicLyrics
  related: YouTubeMusicPage
}

/** The song playing: its id, title, artists (and their page), album (and its page), cover, length, kind, like,
 * and whether it has a music video to show. */
export interface YouTubeMusicTrack {
  video: string
  title: string
  artists: string
  artist?: YouTubeMusicBrowse
  album: string
  albumPage?: YouTubeMusicBrowse
  image?: string
  duration: number
  kind: YouTubeMusicKind
  like: YouTubeMusicRating
  hasVideo: boolean
}

/** What YouTube Music plays: the song, playing (and waiting for its sound), how far, the options, the volume,
 * the Song/Video switch, the queue around the song (`queue` from the whole queue's `start`; `index`: the
 * song's place in the whole queue), what it plays from, and whether an ad plays. */
export interface YouTubeMusicPlayback {
  track: YouTubeMusicTrack
  playing: boolean
  loading: boolean
  position: number
  duration: number
  shuffle: boolean
  repeat: YouTubeMusicRepeat
  volume: number | null
  video: boolean
  queue: YouTubeMusicItem[]
  start: number
  index: number
  from: string
  ad: boolean
}

/** What the YouTube app asks of YouTube's player (hidden, under the page) or of YouTube (`westplay_host::
 * youtube::Command`). `show` places the video, in the frame's pixels. */
export type YouTubeCommand =
  | { action: 'load'; video: string; start?: number }
  | { action: 'play' }
  | { action: 'pause' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'captions'; track: string | null }
  | { action: 'quality'; quality: string }
  | { action: 'rate'; rate: number }
  | { action: 'audioTrack'; id: string }
  | { action: 'stableVolume'; on: boolean }
  | { action: 'voiceBoost'; on: boolean }
  | { action: 'show'; x: number; y: number; width: number; height: number; crop?: boolean }
  | { action: 'hide' }
  | { action: 'close' }
  | { action: 'language'; code: string }
  | { action: 'search'; id: number; query: string; more?: string; filter?: 'channels' | 'playlists' }
  | { action: 'playlist'; id: number; playlist: string; more?: string }
  | { action: 'channel'; id: number; channel: string; more?: string }
  | { action: 'subscribe'; id: number; channel: string; on: boolean }
  | { action: 'suggest'; id: number; query: string }
  | { action: 'related'; id: number; video: string }
  | { action: 'feed'; id: number; which: 'home' | 'subscriptions'; more?: string }
  | { action: 'signIn' }
  | { action: 'signOut' }

/** How YouTube's player plays. `state`: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued. */
export interface YouTubeState {
  video: string | null
  title: string
  author: string
  live: boolean
  state: number
  /** Seconds. */
  time: number
  duration: number
  volume: number
  ad: boolean
  /** The video's captions (YouTube's id for each: `.en`, `a.en` automatic) and the one shown (`null`: none). */
  captions: { track: string; name: string }[]
  caption: string | null
  /** The qualities the video has, best first (`hd1080`, "1080p60"); the one chosen (`auto`: YouTube's own
   * choice), and the one playing ("720p"). */
  qualities: { quality: string; label: string }[]
  quality: string
  qualityLabel: string
  /** The speeds it can play at and the one it plays at; its audio tracks (none: only its own) and the one
   * playing; YouTube's Stable volume and Voice boost (`null`: the video doesn't offer it). */
  rates: number[]
  rate: number
  audioTracks: { id: string; name: string; original: boolean }[]
  audioTrack: string | null
  stableVolume: boolean | null
  voiceBoost: boolean | null
}

/** A video as the YouTube app lists it; its texts are YouTube's own, in the cabin's language. */
export interface YouTubeVideo {
  video: string
  title: string
  channel: string
  avatar: string | null
  thumbnail: string | null
  /** "13:34"; empty for a live video. */
  duration: string
  /** "17K", "14 Mn" */
  views: string
  /** "2 days ago" */
  age: string
  live: boolean
  /** Its channel's id, to open the channel (absent: not known). */
  channelId?: string
}

/** A channel; its texts are YouTube's ("@LofiGirl", "15.8M subscribers", "468 videos"). */
export interface YouTubeChannel {
  channel: string
  name: string
  avatar: string | null
  banner: string | null
  handle: string
  subscribers: string
  videos: string
  /** Whether the signed-in player is subscribed (`null`: not signed in, or not known). */
  subscribed?: boolean | null
}

/** A playlist; its texts are YouTube's ("13 videos", "2M views"). In a search, `after`: how many of the
 * search's videos come before it. */
export interface YouTubePlaylist {
  playlist: string
  title: string
  channel: string
  channelId?: string
  thumbnail: string | null
  count: string
  views: string
  after: number
}

/** YouTube's answer: videos (and the token for more), suggestions, or that it didn't answer. */
export interface YouTubeAnswer {
  videos?: YouTubeVideo[]
  more?: string | null
  suggestions?: string[]
  /** Channels a search found; the channel a channel's page is. */
  channels?: YouTubeChannel[]
  channel?: YouTubeChannel | null
  /** Playlists a search found; the playlist a playlist's page is. */
  playlists?: YouTubePlaylist[]
  playlist?: YouTubePlaylist | null
  failed?: boolean
}

/** Text typed, or a key with a meaning of its own. */
export type TypedEvent = { text: string } | { key: 'backspace' | 'wordBackspace' | 'enter' | 'escape' }

/** Whether the player's own keyboard types into the field selected: no field selected (`off`), typing, or
 * the keys the game's (`paused`: the truck moved, or the field was selected while it moved). */
export type TypingState = 'off' | 'typing' | 'paused'

/** What the page asks of the game's route: a stop added (before a job's destination), the route to a place
 * alone (not on a job), or no more stops. */
export type RouteAction = 'addStop' | 'go' | 'end'

/** A route WestPlay planned (crates/host planner.rs): what it's planned for, its length (metres) and time
 * (seconds), both on the map's ground (the page gives them in the game's scale), and its line through its
 * map nodes (`[x, z]`, metres). */
export interface PlannedRoute {
  plan: 'fastest' | 'shortest' | 'smallRoads' | 'other'
  length: number
  seconds: number
  line: [number, number][]
}

/** How the game answered: done, or why not (`noAnswer`: no game to ask, or it didn't answer in time;
 * `elevated`: the game runs as administrator, so WestPlay can't see its route, a `followed` answer's). */
export type RouteAnswer =
  'done' | 'alreadyThere' | 'onJob' | 'full' | 'noSuchNode' | 'unsupported' | 'unexpected' | 'noAnswer' | 'elevated'

export type JobMarketMode = 'freight' | 'cargo' | 'quick'

export type JobMarketAnswer =
  | 'ok'
  | 'unsupported'
  | 'multiplayer'
  | 'onJob'
  | 'noTruck'
  | 'noTrailer'
  | 'offerExpired'
  | 'notReady'
  | 'failed'
  | 'noAnswer'

export interface JobMarketOffer {
  mode: JobMarketMode
  companyPtr: string
  offerPtr: string
  originCompanyId: string
  originCompany: string
  originCityId: string
  originCity: string
  destCompanyId: string
  destCompany: string
  destCityId: string
  destCity: string
  cargoId: string
  cargoName: string
  cargoIcon?: string
  distanceKm: number
  ferryTime: number
  ferryPrice: number
  urgency: number
  expiresInMin: number
  unitsCount?: number
  massKg: number
  reward: number
  fragile: boolean
  fragilityPct?: number
  valuable: boolean
  adrClass: number
  overweight: boolean
  oversize: boolean
  companyTruck?: string
  trailerDef?: string
  distFromPlayerKm?: number
}

export interface JobMarketSnapshot {
  gameTime: number
  onJob: boolean
  hasOwnTruck: boolean
  hasOwnTrailer: boolean
  trailerAttached: boolean
  quickSupported: boolean
  offers: JobMarketOffer[]
}

interface WebViewBridge {
  addEventListener(type: 'message', listener: (event: { data: HostMessage }) => void): void
  removeEventListener(type: 'message', listener: (event: { data: HostMessage }) => void): void
  postMessage(message: PageMessage): void
}

declare global {
  interface Window {
    chrome?: { webview?: WebViewBridge }
    /** The phone page (phone/socket.ts): its host is the PC it was served from, not WebView2's. */
    westplayPhone?: boolean
    /** The phone in the game's page (handset.html): beside the cab's, on its browser. */
    westplayHandset?: boolean
  }
}

/** The cabin page as a live demo in the companion's onboarding (`cabin.html?demo`): never the host's page,
 * even inside a WebView2 (the companion's own). See cabin/demo.ts. */
export const IS_DEMO = typeof location !== 'undefined' && new URLSearchParams(location.search).has('demo')

/**
 * The first id of the page's requests to the host (the music apps' questions, the Maps app's routes): the phone
 * page's from far above the cab's, so the answers each gets, the cab's and the phone's alike, are told apart by id.
 */
export function firstRequestId(): number {
  if (typeof window === 'undefined') return 1
  // The phone in the game's page on the player's phone is both: apart from either.
  if (window.westplayHandset && window.westplayPhone) return 1_500_000_000
  return window.westplayHandset ? 2_000_000_000 : window.westplayPhone ? 1_000_000_000 : 1
}

/** Whether the page runs inside the WestPlay host (and not in a plain browser preview, nor as the demo). */
export function isInHost(): boolean {
  return !IS_DEMO && window.chrome?.webview !== undefined
}

/**
 * Whether YouTube's picture is under this page: the cab's alone, inside the host. Not on a player's phone (its way
 * to the PC stands in for WebView2's: isInHost) nor the phone in the game, which have nothing under them (owner
 * 2026-11-03: the phone's panel was cut open to nothing).
 */
export function hasVideoUnder(): boolean {
  return isInHost() && !window.westplayPhone && !window.westplayHandset
}

/** Whether the page is one of the phones' (a player's, or the phone in the game): YouTube's picture can't show there. */
export function isPhonePage(): boolean {
  return window.westplayPhone === true || window.westplayHandset === true
}

/**
 * Calls `listener` for every host message. Returns the unsubscribe function.
 *
 * Subscribing also tells the host the page is ready, so it sends the current state again: anything it
 * sent before this page (re)loaded was lost, and some state (like the truck) is only sent on change.
 */
export function subscribeHostMessages(listener: (message: HostMessage) => void): () => void {
  const webview = window.chrome?.webview
  if (!webview) return () => {}
  const onMessage = (event: { data: HostMessage }) => listener(event.data)
  webview.addEventListener('message', onMessage)
  webview.postMessage({ type: 'ready' })
  return () => webview.removeEventListener('message', onMessage)
}

/** Sends a message to the host (nothing happens in a plain browser preview). */
export function postToHost(message: PageMessage): void {
  window.chrome?.webview?.postMessage(message)
}

/** A truck with no identity (the SDK reports empty strings when there is no truck). */
export function hasTruck(truck: TruckInfo | null): truck is TruckInfo {
  return truck !== null && truck.brandId !== ''
}

/** Which of the game's virtual mirrors show (crates/host settings.rs `VirtualMirrors`): the reversing camera is
 * made from the left one whatever is chosen. */
export type VirtualMirrors = 'none' | 'right' | 'left' | 'both'

/** How low the truck's front may go (crates/host settings.rs `LowFront`): the game's own, lower, the lowest. */
export type LowFront = 'normal' | 'low' | 'lowest'

/** How firm the dampers are (crates/host settings.rs `Damping`). */
export type Damping = 'soft' | 'normal' | 'firm' | 'firmest'
/** How big the mouse's dot is (the phones' Settings). */
export type CursorSize = 'small' | 'normal' | 'large' | 'larger'
/** Where media audio plays (`westplay_host::settings::AudioOutput`). */
export type AudioOutput = 'cabin' | 'device'

/** How soon the brakes fade (`westplay_host::brake_fade::Level`). */
export type BrakeFadeLevel = 'light' | 'realistic' | 'hard'
/** Where the brakes' HUD sits, and how big. */
export type BrakeHudCorner = 'bottomLeft' | 'bottomRight' | 'topLeft' | 'topRight'
export type BrakeHudSize = 'small' | 'medium' | 'large'
