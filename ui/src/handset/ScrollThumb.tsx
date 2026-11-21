import { type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react'

/** How often the page's size is looked at again (ms): an app's rows come and go as the game says. */
const LOOK_EVERY = 300
/** The thumb's least height (px), so a finger finds it on a long page. */
const LEAST_THUMB = 56

interface Metrics {
  /** The scrolled box's place in the screen (px). */
  top: number
  height: number
  /** The thumb's place along it and its size (px). */
  thumbTop: number
  thumbHeight: number
}

/**
 * A scroll bar at the screen's right edge, to drag (owner 2026-11-20: "on a page full of buttons I have to find a
 * place without one to drag it down; a bar in the corner, as the browsers have"). The buttons held for as long as
 * they're pressed (Commands, Suspensão) take the finger and don't let the page move, by design: a drag starting on
 * one would press it. Shown only while the page is longer than the screen; a tap above or below the thumb moves a
 * screen's height.
 */
export function ScrollThumb({ target }: { target: RefObject<HTMLElement | null> }) {
  const [metrics, setMetrics] = useState<Metrics | null>(null)
  const [dragging, setDragging] = useState(false)
  const drag = useRef<{ y: number; scroll: number; perPixel: number } | null>(null)

  useEffect(() => {
    const box = target.current
    if (!box) return
    const measure = () => {
      const range = box.scrollHeight - box.clientHeight
      if (range <= 4) {
        setMetrics(null)
        return
      }
      const track = box.clientHeight
      const thumbHeight = Math.max(LEAST_THUMB, (track * box.clientHeight) / box.scrollHeight)
      const thumbTop = (box.scrollTop / range) * (track - thumbHeight)
      const next = { top: box.offsetTop, height: track, thumbTop, thumbHeight }
      setMetrics((now) =>
        now &&
        Math.abs(now.top - next.top) < 0.5 &&
        Math.abs(now.height - next.height) < 0.5 &&
        Math.abs(now.thumbTop - next.thumbTop) < 0.5 &&
        Math.abs(now.thumbHeight - next.thumbHeight) < 0.5
          ? now
          : next,
      )
    }
    measure()
    box.addEventListener('scroll', measure, { passive: true })
    const timer = setInterval(measure, LOOK_EVERY)
    return () => {
      box.removeEventListener('scroll', measure)
      clearInterval(timer)
    }
  }, [target])

  if (!metrics) return null
  const box = target.current

  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (!box) return
    event.preventDefault()
    event.stopPropagation()
    const range = box.scrollHeight - box.clientHeight
    const travel = Math.max(1, metrics.height - metrics.thumbHeight)
    // Where in the track it was touched: on the thumb, the thumb is taken; above or below it, a screen's height.
    const at = event.clientY - event.currentTarget.getBoundingClientRect().top
    if (at < metrics.thumbTop || at > metrics.thumbTop + metrics.thumbHeight) {
      const way = at < metrics.thumbTop ? -1 : 1
      box.scrollBy({ top: way * box.clientHeight * 0.9, behavior: 'smooth' })
      return
    }
    event.currentTarget.setPointerCapture?.(event.pointerId)
    drag.current = { y: event.clientY, scroll: box.scrollTop, perPixel: range / travel }
    setDragging(true)
  }
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const from = drag.current
    if (!from || !box) return
    box.scrollTop = from.scroll + (event.clientY - from.y) * from.perPixel
  }
  const up = () => {
    drag.current = null
    setDragging(false)
  }

  return (
    <div
      className={`ios-scroll-track${dragging ? ' dragging' : ''}`}
      style={{ top: metrics.top, height: metrics.height }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
      onLostPointerCapture={up}
      aria-hidden
    >
      <div className="ios-scroll-thumb" style={{ top: metrics.thumbTop, height: metrics.thumbHeight }} />
    </div>
  )
}
