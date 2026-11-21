/**
 * The player's own phone as the truck's steering wheel (owner 2026-11-19: "sem delay, como se fosse um volante
 * mesmo"), served by the host on the PC's network over HTTPS (crates/host remote.rs and wheel.rs): the phone's
 * gyroscope steers, the pedals on the screen (or tilting the phone) set the throttle and the brake, and a little
 * wheel in the middle turns as the truck's will, to get a feel for it. What it sends is shaped already by the
 * player's settings (shape.ts), saved on the phone; some 60 times a second, on a WebSocket of its own; hidden or
 * gone, it all goes back to 0 at once (and the host lets go of it if it goes quiet).
 */
import {
  DEFAULTS,
  LIMITS,
  type PedalMode,
  type WheelSettings,
  encode,
  readSettings,
  shapePedal,
  shapeSteering,
  smooth,
  tiltToPedals,
  touchToPedal,
  wrap,
} from './shape'
import { Orientation } from './sensor'
import { WORDS as w } from './words'
import './wheel.css'

const SETTINGS_KEY = 'maxdevmod-wheel-settings'
const KEPT_KEY = 'maxdevmod-wheel-key'
/** The HTTPS port (crates/host remote.rs `TLS_PORT`). */
const TLS_PORT = 47801
/** Sent at most this often (ms), and at least this often while nothing changes (the host lets go after 250 ms). */
const SEND_EVERY = 14
const KEEP_ALIVE = 80

type Pedal = 'throttle' | 'brake'

let settings: WheelSettings = readSettings(localStorage.getItem(SETTINGS_KEY))

const key = (() => {
  const given = decodeURIComponent(location.hash.replace(/^#/, '')).trim()
  if (/^[A-Za-z0-9]{16,}$/.test(given)) {
    localStorage.setItem(KEPT_KEY, given)
    return given
  }
  return localStorage.getItem(KEPT_KEY)
})()

// ---- The page ----

const root = document.getElementById('app')!
root.innerHTML = `
  <main class="wheel-page">
    <div class="pedal brake" data-pedal="brake">
      <div class="pedal-fill"></div>
      <div class="pedal-plate"><span class="pedal-grip"></span><span class="pedal-name">${w.brake}</span><span class="pedal-value"></span></div>
    </div>
    <section class="middle">
      <header class="bar">
        <span class="status"><i class="dot"></i><span class="status-text">${w.connecting}</span></span>
        <span class="bar-buttons">
          <button class="button centre" type="button">${w.centre}</button>
          <button class="button fill-screen" type="button" aria-label="${w.fillScreen}">⛶</button>
          <button class="button open-settings" type="button" aria-label="${w.settings}">⚙</button>
        </span>
      </header>
      <div class="wheel-holder">
        <svg class="wheel" viewBox="-100 -100 200 200" aria-hidden="true">
          <circle r="88" class="rim-shadow"/>
          <circle r="84" class="rim"/>
          <circle r="84" class="rim-inner"/>
          <path d="M -80 6 Q -40 20 -16 14 L -16 -4 Q -40 -2 -80 -12 Z" class="spoke"/>
          <path d="M 80 6 Q 40 20 16 14 L 16 -4 Q 40 -2 80 -12 Z" class="spoke"/>
          <path d="M -10 16 L -14 80 L 14 80 L 10 16 Z" class="spoke"/>
          <circle r="26" class="hub"/>
          <text y="7" class="hub-mark">M</text>
          <rect x="-5" y="-92" width="10" height="16" rx="3" class="top-mark"/>
        </svg>
      </div>
      <div class="truck-row">
        <button class="button ignition" type="button">${w.engineStart}</button>
        <span class="gear" aria-label="${w.gear}">–</span>
        <span class="gears">
          <button class="button gear-button" type="button" data-command="gearreverse">R</button>
          <button class="button gear-button" type="button" data-command="gear0">N</button>
          <button class="button gear-button" type="button" data-command="geardrive">D</button>
        </span>
        <span class="steer-value">0%</span>
      </div>
      <div class="truck-row small">
        <button class="button cmd blink" type="button" data-command="lblinker" data-state="left" aria-label="${w.leftBlinker}">◀</button>
        <button class="button cmd hazard" type="button" data-command="flasher4way" aria-label="${w.hazard}">⚠</button>
        <button class="button cmd blink" type="button" data-command="rblinker" data-state="right" aria-label="${w.rightBlinker}">▶</button>
        <button class="button cmd parking" type="button" data-command="parkingbrake" data-state="parking" aria-label="${w.parkingBrake}">(P)</button>
        <button class="button cmd lights" type="button" data-command="light" data-state="low">${w.lights}</button>
        <button class="button cmd lights" type="button" data-command="hblight" data-state="high">${w.highBeam}</button>
        <button class="button cmd" type="button" data-command="ignitionon" data-state="electric">${w.ignitionKey}</button>
        <button class="button cmd" type="button" data-command="camreset">${w.cameraReset}</button>
        <button class="button cmd" type="button" data-command="looksteer">${w.cameraFollow}</button>
      </div>
    </section>
    <div class="pedal throttle" data-pedal="throttle">
      <div class="pedal-fill"></div>
      <div class="pedal-plate"><span class="pedal-grip"></span><span class="pedal-name">${w.throttle}</span><span class="pedal-value"></span></div>
    </div>
  </main>
  <div class="overlay start-overlay">
    <div class="overlay-card">
      <h1>${w.title}</h1>
      <p class="start-message">${w.startHint}</p>
      <button class="button big start" type="button">${w.start}</button>
      <a class="button big https-link hidden" href="#">${w.openHttps}</a>
    </div>
  </div>
  <div class="rotate-hint"><div>⟳</div><p>${w.rotate}</p></div>
  <aside class="settings hidden" aria-label="${w.settings}">
    <header class="settings-head">
      <strong>${w.settings}</strong>
      <span class="settings-live"></span>
      <button class="button reset-top" type="button">${w.reset}</button>
      <button class="button close-settings" type="button">${w.close}</button>
    </header>
    <div class="settings-body"></div>
  </aside>
`

const $ = <T extends Element = HTMLElement>(selector: string) => root.querySelector(selector) as T
const page = $('.wheel-page')
const wheel = $<SVGElement>('.wheel')
const steerValue = $('.steer-value')
const statusText = $('.status-text')
const dot = $('.dot')
const pedals: Record<Pedal, HTMLElement> = { throttle: $('.pedal.throttle'), brake: $('.pedal.brake') }
const startOverlay = $('.start-overlay')
const startMessage = $('.start-message')
const settingsPanel = $('.settings')
const settingsLive = $('.settings-live')
const ignition = $('.ignition')
const gearShown = $('.gear')

function applyLook() {
  document.body.classList.toggle('left-handed', settings.leftHanded)
  document.body.classList.toggle('big-buttons', settings.bigButtons)
  // In percent of the page's width, which stays the long side when the page is turned back (keepTurn).
  pedals.throttle.style.flexBasis = `${settings.throttleWidth}%`
  pedals.brake.style.flexBasis = `${settings.brakeWidth}%`
  for (const pedal of Object.values(pedals)) pedal.style.height = `${settings.pedalHeight}%`
  document.body.classList.toggle('high-contrast', settings.highContrast)
  document.body.classList.toggle('tilt-mode', settings.pedalMode === 'tilt')
}
applyLook()

if (key) {
  const manifest = document.createElement('link')
  manifest.rel = 'manifest'
  manifest.href = `/app.webmanifest?page=wheel&k=${encodeURIComponent(key)}&name=${encodeURIComponent(`${w.title} MaxDevMod`)}&short=${encodeURIComponent(w.title)}`
  document.head.appendChild(manifest)
}

// ---- What the phone feels ----

const orientation = new Orientation()
let centreTurn: number | null = null
let centreTilt = 0
let centreWanted = true
let motionSeen = false

function onMotion(event: DeviceMotionEvent) {
  const gravity = event.accelerationIncludingGravity
  if (!gravity || gravity.x === null || gravity.y === null || gravity.z === null) return
  const rate = event.rotationRate?.alpha
  orientation.update(gravity.x, gravity.y, gravity.z, typeof rate === 'number' ? rate : null, performance.now())
  motionSeen = true
  if (centreWanted && orientation.turn !== null) centre()
  pump()
}

function centre() {
  if (orientation.turn === null) {
    centreWanted = true
    return
  }
  centreWanted = false
  centreTurn = orientation.turn
  centreTilt = orientation.tilt ?? 0
  steer = 0
}

/** The touches on the pedals: which pedal, and how far down it (0 its top, 1 its bottom). */
const touches = new Map<number, { pedal: Pedal; fromTop: number }>()

/** How far down a pedal a touch is (0 its top, 1 its bottom), in the pedal's own frame (the page may be turned). */
function touchAt(pedal: Pedal, clientX: number, clientY: number) {
  const element = pedals[pedal]
  const box = element.getBoundingClientRect()
  const [x, y] = [clientX - (box.left + box.width / 2), clientY - (box.top + box.height / 2)]
  const r = (-turned * Math.PI) / 180
  const down = x * Math.sin(r) + y * Math.cos(r)
  const height = Math.max(1, element.offsetHeight)
  return Math.min(1, Math.max(0, down / height + 0.5))
}

for (const pedal of ['throttle', 'brake'] as Pedal[]) {
  const element = pedals[pedal]
  element.addEventListener('pointerdown', (event) => {
    event.preventDefault()
    try {
      element.setPointerCapture?.(event.pointerId)
    } catch {
      // A pointer already gone: its touch is kept all the same, until it lifts.
    }
    touches.set(event.pointerId, { pedal, fromTop: touchAt(pedal, event.clientX, event.clientY) })
    buzz(1)
    pump(true)
  })
  element.addEventListener('pointermove', (event) => {
    const touch = touches.get(event.pointerId)
    if (!touch) return
    touch.fromTop = touchAt(touch.pedal, event.clientX, event.clientY)
    pump()
  })
  const lift = (event: PointerEvent) => {
    if (touches.delete(event.pointerId)) pump(true)
  }
  element.addEventListener('pointerup', lift)
  element.addEventListener('pointercancel', lift)
  element.addEventListener('lostpointercapture', lift)
}

// ---- Shaped and sent ----

let steer = 0
let throttle = 0
let brake = 0
let lastShaped = performance.now()
let atLock = false
let brakeFull = false

function pressedBy(pedal: Pedal, mode: PedalMode): number {
  let pressed = 0
  for (const touch of touches.values()) {
    if (touch.pedal !== pedal) continue
    pressed = Math.max(pressed, mode === 'onoff' ? 1 : touchToPedal(touch.fromTop))
  }
  return pressed
}

function shape(now: number) {
  const ms = now - lastShaped
  lastShaped = now
  const turned = orientation.turn === null || centreTurn === null ? 0 : wrap(orientation.turn - centreTurn)
  steer = smooth(steer, shapeSteering(turned, settings), settings.smoothing, ms)
  if (Math.abs(steer) < 0.0005) steer = 0
  let throttlePressed = pressedBy('throttle', settings.pedalMode)
  let brakePressed = pressedBy('brake', settings.pedalMode)
  if (settings.pedalMode === 'tilt' && orientation.tilt !== null) {
    const tilted = tiltToPedals(orientation.tilt - centreTilt, settings)
    throttlePressed = Math.max(throttlePressed, tilted.throttle)
    brakePressed = Math.max(brakePressed, tilted.brake)
  }
  throttle = shapePedal(throttlePressed, settings.throttleSensitivity, settings.throttleCurve)
  brake = shapePedal(brakePressed, settings.brakeSensitivity, settings.brakeCurve)
  const lock = Math.abs(steer) > 0.995
  if (lock && !atLock) buzz(2)
  atLock = lock
  const full = brake > 0.995
  if (full && !brakeFull) buzz(1.5)
  brakeFull = full
}

let socket: WebSocket | null = null
let lastSent = 0
let lastText = ''
let released = true

function send(text: string) {
  if (socket?.readyState === WebSocket.OPEN) socket.send(text)
}

/** Shapes what the phone feels now and sends it (at most every `SEND_EVERY` ms, unless `now` it must). */
function pump(force = false) {
  if (!running || document.hidden) return
  const now = performance.now()
  shape(now)
  if (!force && now - lastSent < SEND_EVERY) return
  const text = encode(steer, throttle, brake, settings.gameSteering === 'absolute')
  if (!force && text === lastText && now - lastSent < KEEP_ALIVE) return
  lastSent = now
  lastText = text
  released = false
  send(text)
}

/** Everything let go at once: the pedals up, the steering straight, the host told. */
function release() {
  touches.clear()
  throttle = 0
  brake = 0
  if (!released) {
    send('w 0 0 0')
    send('0')
  }
  released = true
  lastText = ''
}

setInterval(() => pump(), KEEP_ALIVE / 2)

// ---- The connection ----

let latency: number | null = null

function connect() {
  if (!key) return
  const scheme = location.protocol === 'https:' ? 'wss' : 'ws'
  const opened = new WebSocket(`${scheme}://${location.host}/ws?k=${encodeURIComponent(key)}&wheel=1`)
  socket = opened
  showStatus('connecting')
  opened.onopen = () => {
    showStatus('connected')
    lastText = ''
    pump(true)
  }
  opened.onmessage = (event) => {
    const text = typeof event.data === 'string' ? event.data : ''
    if (text.startsWith('s ')) showTruck(text)
    if (text.startsWith('P ')) {
      const sentAt = Number(text.slice(2))
      if (Number.isFinite(sentAt)) {
        const measured = performance.now() - sentAt
        latency = latency === null ? measured : latency * 0.7 + measured * 0.3
        showStatus('connected')
      }
    }
  }
  opened.onclose = () => {
    if (socket !== opened) return
    socket = null
    latency = null
    showStatus('offline')
    setTimeout(connect, 1000)
  }
}

setInterval(() => send(`p ${performance.now().toFixed(1)}`), 1000)

function showStatus(state: 'connecting' | 'connected' | 'offline') {
  dot.className = `dot ${state}`
  statusText.textContent =
    state === 'connected'
      ? `${w.connected}${latency === null ? '' : ` · ${Math.round(latency)} ${w.ms}`}`
      : state === 'offline'
        ? w.offline
        : w.connecting
  dot.classList.toggle('slow', state === 'connected' && latency !== null && latency > 40)
}

// ---- Drawn ----

function draw() {
  wheel.style.transform = `rotate(${(steer * settings.drawnLock).toFixed(2)}deg)`
  steerValue.textContent = `${steer < -0.005 ? '◀ ' : ''}${Math.round(Math.abs(steer) * 100)}%${steer > 0.005 ? ' ▶' : ''}`
  for (const [pedal, value] of [
    ['throttle', throttle],
    ['brake', brake],
  ] as [Pedal, number][]) {
    const element = pedals[pedal]
    element.style.setProperty('--pressed', value.toFixed(3))
    element.classList.toggle('down', value > 0.01)
    element.querySelector('.pedal-value')!.textContent = value > 0.01 ? `${Math.round(value * 100)}%` : ''
  }
  if (!settingsPanel.classList.contains('hidden')) {
    settingsLive.textContent = `${w.steeringGroup} ${Math.round(steer * 100)}% · ${w.throttle} ${Math.round(throttle * 100)}% · ${w.brake} ${Math.round(brake * 100)}%`
  }
  requestAnimationFrame(draw)
}
requestAnimationFrame(draw)

// ---- Haptics ----

// ---- The truck's buttons (owner 2026-11-19: "um botão de ligar e desligar o caminhão e ... engatar a marcha para
// frente e para trás"): the game's own controls, as the phones' Commands app gives them (crates/host remote.rs). ----

/** The truck's state as the host last said (`s <engine> <electric> <gear>`). */
let engineOn = false

function showTruck(text: string) {
  const [, engine, electric, gear, parking, low, high, left, right] = text.split(' ')
  const state: Record<string, boolean> = {
    electric: electric === '1',
    parking: parking === '1',
    low: low === '1',
    high: high === '1',
    left: left === '1',
    right: right === '1',
  }
  for (const button of root.querySelectorAll<HTMLButtonElement>('.cmd[data-state]')) {
    button.classList.toggle('on', state[button.dataset.state ?? ''] ?? false)
  }
  engineOn = engine === '1'
  const number = Number(gear)
  ignition.textContent = engineOn ? w.engineStop : w.engineStart
  ignition.classList.toggle('on', engineOn)
  gearShown.textContent = !Number.isFinite(number) ? '–' : number > 0 ? `D${number}` : number < 0 ? `R${-number}` : 'N'
  gearShown.classList.toggle('reverse', number < 0)
}

function command(name: string, down: boolean) {
  send(JSON.stringify({ type: 'truckCommand', command: name, down }))
}

/** A command pressed and let go after `ms`. */
function tap(name: string, ms = 200) {
  command(name, true)
  setTimeout(() => command(name, false), ms)
}

// The engine: off, held to start (the key turned to start, at least a second and a half, as long as it's held);
// on, a tap switches it off.
let starting: number | null = null
ignition.addEventListener('pointerdown', (event) => {
  event.preventDefault()
  buzz(1)
  if (engineOn) {
    tap('ignitionoff')
    return
  }
  starting = performance.now()
  command('ignitionstrt', true)
})
const letKeyGo = () => {
  if (starting === null) return
  const held = performance.now() - starting
  starting = null
  setTimeout(() => command('ignitionstrt', false), Math.max(0, 1500 - held))
}
ignition.addEventListener('pointerup', letKeyGo)
ignition.addEventListener('pointercancel', letKeyGo)
ignition.addEventListener('pointerleave', letKeyGo)

for (const button of root.querySelectorAll<HTMLButtonElement>('.gear-button, .cmd')) {
  button.addEventListener('pointerdown', (event) => {
    event.preventDefault()
    buzz(1)
    tap(button.dataset.command ?? '')
  })
}

function buzz(times: number) {
  if (!settings.vibration || typeof navigator.vibrate !== 'function') return
  navigator.vibrate(Math.round([8, 15, 25][settings.vibrationStrength - 1] * times))
}

// ---- Starting ----

let running = false
let wakeLock: { release: () => Promise<void> } | null = null

async function keepAwake() {
  try {
    const wake = (
      navigator as Navigator & { wakeLock?: { request: (kind: 'screen') => Promise<{ release: () => Promise<void> }> } }
    ).wakeLock
    wakeLock = (await wake?.request('screen')) ?? null
  } catch {
    wakeLock = null
  }
}

async function start() {
  // First, while the tap still counts as the player's (a permission asked below may use it up).
  if (settings.fillScreen) fillScreen()
  const motion = DeviceMotionEvent as unknown as { requestPermission?: () => Promise<string> }
  if (typeof motion?.requestPermission === 'function') {
    // Refused, the pedals still work: the page goes on, and says so (below, no motion seen).
    try {
      await motion.requestPermission()
    } catch {
      // As refused.
    }
  }
  window.addEventListener('devicemotion', onMotion)
  running = true
  document.body.classList.add('running')
  turnedFrom = screenAngle()
  keepTurn()
  centreWanted = true
  startOverlay.classList.add('hidden')
  void keepAwake()
  setTimeout(() => {
    if (!motionSeen) {
      // No sensor: said, and a tap goes on with the pedals alone.
      startOverlay.classList.remove('hidden')
      startMessage.textContent = w.noSensor
      $('.start').classList.add('hidden')
    }
  }, 2000)
}

startOverlay.addEventListener('click', () => {
  if (running) startOverlay.classList.add('hidden')
})

// ---- Filling the screen (owner 2026-11-19: "a opção de preencher a tela") ----

/** Whether the browser can put a page in full screen (an iPhone's Safari can't: the home screen's app can). */
const canFill = typeof document.documentElement.requestFullscreen === 'function'

/** Full screen and landscape held, where the phone lets the page: not waited for. */
function fillScreen() {
  if (!canFill || document.fullscreenElement) return
  document.documentElement
    .requestFullscreen({ navigationUI: 'hide' })
    .then(() => lockTurn())
    .catch(() => {})
}

// ---- The page held still (owner 2026-11-20: "se eu girar o celular muito, a página fica rodando") ----
// Turned like a wheel, the phone rotates the page as it would any other (to portrait, or upside down). Where the
// browser lets it (full screen, an Android phone's), the page is locked in the exact way it started (not just
// "landscape", which still flips); elsewhere (an iPhone's Safari), the page is turned back by as much as the browser
// turned it, so it stays put on the phone. The motion sensor counts in the phone's own axes: steering isn't touched.

/** The way the browser shows the page, in degrees (0, 90, 180, 270). */
function screenAngle(): number {
  const angle = screen.orientation?.angle ?? Number((window as Window & { orientation?: number }).orientation ?? 0)
  return (((Math.round(angle / 90) * 90) % 360) + 360) % 360
}

/** The angle the page started at, and how much it's turned back now (CSS degrees). */
let turnedFrom = 0
let turned = 0

function lockTurn() {
  const type = screen.orientation?.type
  const lock = (screen.orientation as ScreenOrientation & { lock?: (o: string) => Promise<void> }).lock
  void lock?.call(screen.orientation, type?.startsWith('landscape') ? type : 'landscape').catch(() => {})
}

function keepTurn() {
  if (!running) return
  turned = (((screenAngle() - turnedFrom) % 360) + 360) % 360
  document.body.style.setProperty('--turn', `${turned}deg`)
  document.body.classList.toggle('turned-side', turned === 90 || turned === 270)
  document.body.classList.toggle('turned-over', turned === 180)
}

screen.orientation?.addEventListener?.('change', keepTurn)
window.addEventListener('orientationchange', () => setTimeout(keepTurn, 50))
window.addEventListener('resize', keepTurn)

$('.fill-screen').addEventListener('click', () => {
  if (!canFill) {
    window.alert(w.fillScreenIphone)
  } else if (document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {})
    settings = { ...settings, fillScreen: false }
    save()
  } else {
    settings = { ...settings, fillScreen: true }
    save()
    fillScreen()
  }
})

// Full screen left (the phone's back gesture, a notification): the next touch fills it again, if the player wants it.
document.addEventListener(
  'pointerdown',
  () => {
    if (running && settings.fillScreen) fillScreen()
  },
  { capture: true },
)

if (!key) {
  startMessage.textContent = w.noKey
  $('.start').classList.add('hidden')
} else if (!window.isSecureContext) {
  startMessage.textContent = w.noHttps
  const link = $<HTMLAnchorElement>('.https-link')
  link.href = `https://${location.hostname}:${TLS_PORT}/wheel.html#${key}`
  link.classList.remove('hidden')
  $('.start').classList.add('hidden')
  connect()
} else {
  connect()
}

$('.start').addEventListener('click', () => void start())
$('.centre').addEventListener('click', () => {
  centre()
  buzz(1)
  pump(true)
})

document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    release()
  } else {
    if (running) void keepAwake()
    pump(true)
  }
})
window.addEventListener('pagehide', release)
window.addEventListener('blur', () => {
  if (touches.size) {
    touches.clear()
    pump(true)
  }
})
void wakeLock

// ---- Settings ----

type Item =
  | { kind: 'range'; key: keyof typeof LIMITS; label: string; show: (value: number) => string }
  | {
      kind: 'toggle'
      key: { [K in keyof WheelSettings]: WheelSettings[K] extends boolean ? K : never }[keyof WheelSettings]
      label: string
    }
  | { kind: 'mode'; label: string; key: 'pedalMode' | 'gameSteering'; options: [string, string][] }
  | { kind: 'note'; text: string; when?: () => boolean }

const times = (value: number) => `${value.toFixed(2)}×`
const degrees = (value: number) => `${Math.round(value * 10) / 10}°`
const percent = (value: number) => `${Math.round(value * 100)}%`

const GROUPS: [string, Item[]][] = [
  [
    w.steeringGroup,
    [
      { kind: 'range', key: 'sensitivity', label: w.sensitivity, show: times },
      { kind: 'range', key: 'range', label: w.range, show: degrees },
      { kind: 'range', key: 'smoothing', label: w.smoothing, show: percent },
      { kind: 'range', key: 'deadzone', label: w.deadzone, show: degrees },
      { kind: 'range', key: 'curve', label: w.curve, show: percent },
      { kind: 'toggle', key: 'invert', label: w.invert },
      { kind: 'range', key: 'drawnLock', label: w.drawnLock, show: degrees },
      {
        kind: 'mode',
        label: w.gameSteering,
        key: 'gameSteering',
        options: [
          ['relative', w.gameSteeringRelative],
          ['absolute', w.gameSteeringAbsolute],
        ],
      },
      { kind: 'note', text: w.gameSteeringHint },
    ],
  ],
  [
    w.pedalsGroup,
    [
      {
        kind: 'mode',
        label: w.pedalMode,
        key: 'pedalMode',
        options: [
          ['touch', w.modeTouch],
          ['tilt', w.modeTilt],
          ['onoff', w.modeOnOff],
        ],
      },
      { kind: 'note', text: w.tiltHint, when: () => settings.pedalMode === 'tilt' },
      { kind: 'range', key: 'throttleSensitivity', label: w.throttleSensitivity, show: times },
      { kind: 'range', key: 'throttleCurve', label: w.throttleCurve, show: percent },
      { kind: 'range', key: 'brakeSensitivity', label: w.brakeSensitivity, show: times },
      { kind: 'range', key: 'brakeCurve', label: w.brakeCurve, show: percent },
      { kind: 'range', key: 'throttleWidth', label: w.throttleWidth, show: (value) => `${value}%` },
      { kind: 'range', key: 'brakeWidth', label: w.brakeWidth, show: (value) => `${value}%` },
      { kind: 'range', key: 'pedalHeight', label: w.pedalHeight, show: (value) => `${value}%` },
      { kind: 'range', key: 'tiltRange', label: w.tiltRange, show: degrees },
      { kind: 'toggle', key: 'tiltInvert', label: w.tiltInvert },
    ],
  ],
  [
    w.feelGroup,
    [
      { kind: 'toggle', key: 'vibration', label: w.vibration },
      { kind: 'range', key: 'vibrationStrength', label: w.vibrationStrength, show: (value) => `${value}` },
      { kind: 'note', text: w.noVibration, when: () => typeof navigator.vibrate !== 'function' },
    ],
  ],
  [
    w.accessGroup,
    [
      { kind: 'toggle', key: 'fillScreen', label: w.fillScreen },
      { kind: 'toggle', key: 'leftHanded', label: w.leftHanded },
      { kind: 'toggle', key: 'bigButtons', label: w.bigButtons },
      { kind: 'toggle', key: 'highContrast', label: w.highContrast },
    ],
  ],
]

function save() {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
  applyLook()
}

function buildSettings() {
  const body = $('.settings-body')
  body.replaceChildren()
  for (const [title, items] of GROUPS) {
    const group = document.createElement('section')
    group.className = 'settings-group'
    group.innerHTML = `<h2>${title}</h2>`
    for (const item of items) {
      const row = document.createElement('div')
      row.className = `settings-row ${item.kind}`
      if (item.kind === 'range') {
        const limit = LIMITS[item.key]
        row.innerHTML = `<label><span>${item.label}</span><output></output></label><input type="range" min="${limit.min}" max="${limit.max}" step="${limit.step}">`
        const input = row.querySelector('input')!
        const output = row.querySelector('output')!
        input.value = String(settings[item.key])
        output.textContent = item.show(settings[item.key])
        input.addEventListener('input', () => {
          settings = { ...settings, [item.key]: Number(input.value) }
          output.textContent = item.show(settings[item.key])
          save()
        })
      } else if (item.kind === 'toggle') {
        row.innerHTML = `<label class="toggle"><span>${item.label}</span><input type="checkbox"><i></i></label>`
        const input = row.querySelector('input')!
        input.checked = settings[item.key]
        input.addEventListener('change', () => {
          settings = { ...settings, [item.key]: input.checked }
          save()
          if (item.key === 'vibration' && input.checked) buzz(1)
        })
      } else if (item.kind === 'mode') {
        row.innerHTML = `<span>${item.label}</span><div class="segments">${item.options
          .map(
            ([mode, name]) =>
              `<button type="button" data-mode="${mode}" class="${settings[item.key] === mode ? 'on' : ''}">${name}</button>`,
          )
          .join('')}</div>`
        for (const button of row.querySelectorAll<HTMLButtonElement>('button')) {
          button.addEventListener('click', () => {
            settings = { ...settings, [item.key]: button.dataset.mode } as WheelSettings
            save()
            buildSettings()
          })
        }
      } else {
        if (item.when && !item.when()) continue
        row.textContent = item.text
      }
      group.appendChild(row)
    }
    body.appendChild(group)
  }
  const reset = document.createElement('button')
  reset.type = 'button'
  reset.className = 'button reset'
  reset.textContent = w.reset
  reset.addEventListener('click', resetAll)
  body.appendChild(reset)
}

$('.open-settings').addEventListener('click', () => {
  release()
  buildSettings()
  settingsPanel.classList.remove('hidden')
})
/** Every setting back to the defaults (owner 2026-11-19: for one changed by mistake), once the player confirms. */
function resetAll() {
  if (!window.confirm(w.resetConfirm)) return
  settings = { ...DEFAULTS }
  save()
  buildSettings()
  buzz(1)
}
$('.reset-top').addEventListener('click', resetAll)
$('.close-settings').addEventListener('click', () => settingsPanel.classList.add('hidden'))

// Long presses, zooms and the page's scrolling are off: it's a controller.
document.addEventListener('contextmenu', (event) => event.preventDefault())
document.addEventListener('gesturestart', (event) => event.preventDefault())
void page
