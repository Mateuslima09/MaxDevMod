/**
 * How the page fares on the player's own phone, told to the PC's log (crates/host remote.rs `diag`) every few
 * seconds (owner 2026-11-03: "it stalls, the browser stops, it's too heavy"): the frames it draws, the time its
 * script held the phone, its memory, the page's size, and any error; a phone stalling can't be watched otherwise.
 */

/** How often it's told, ms. */
const EVERY_MS = 4000
/** How long, of each, the frames are counted: asking a frame each frame keeps the phone drawing without end
 * (at 120 a second on the owner's), what's measured made heavier by measuring it. */
const FRAMES_MS = 1000

export function reportHealth(send: (report: object) => void): void {
  let frames = 0
  let busy = 0
  let errors: string[] = []
  let renders = 0
  const started = performance.now()
  let counting = 0
  const count = (now: number) => {
    frames++
    if (now < counting) requestAnimationFrame(count)
  }
  const countFrames = () => {
    frames = 0
    counting = performance.now() + FRAMES_MS
    requestAnimationFrame(count)
  }
  countFrames()
  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) busy += entry.duration
    }).observe({ type: 'longtask', buffered: true })
  } catch {
    // No long tasks in this browser: the frames say it.
  }
  window.addEventListener('error', (event) => errors.push(String(event.message).slice(0, 120)))
  window.addEventListener('unhandledrejection', (event) => errors.push(String(event.reason).slice(0, 120)))
  // How much the page holds: its elements (a list grown without end would show here).
  const observer = new MutationObserver(() => renders++)
  observer.observe(document.body, { childList: true, subtree: true, characterData: true })
  // The touches and what became of them (owner 2026-11-03: "impossible to use"): how many came down, how many
  // made a click, what was under the last, and whether a page's handler kept the browser from it.
  let touches: string[] = []
  const name = (target: EventTarget | null) => {
    const element = target instanceof Element ? target : null
    return element ? `${element.tagName.toLowerCase()}.${String(element.className).split(' ')[0]}` : '?'
  }
  for (const kind of ['pointerdown', 'pointerup', 'pointercancel', 'click', 'touchstart'] as const) {
    window.addEventListener(
      kind,
      (event) => {
        if (touches.length < 12) touches.push(`${kind}:${name(event.target)}${event.defaultPrevented ? '!' : ''}`)
      },
      { capture: false, passive: true },
    )
  }
  setInterval(() => {
    const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory
    send({
      type: 'diag',
      s: Math.round((performance.now() - started) / 1000),
      fps: Math.round((frames * 1000) / FRAMES_MS),
      busyMs: Math.round(busy),
      mutations: renders,
      nodes: document.getElementsByTagName('*').length,
      heapMB: memory ? Math.round(memory.usedJSHeapSize / 1048576) : null,
      screen: `${innerWidth}x${innerHeight}@${devicePixelRatio}`,
      full: document.fullscreenElement !== null,
      hidden: document.hidden,
      errors,
      touches,
    })
    countFrames()
    busy = 0
    renders = 0
    errors = []
    touches = []
  }, EVERY_MS)
}
