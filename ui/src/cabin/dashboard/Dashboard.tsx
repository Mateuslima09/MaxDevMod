import { useLayoutEffect, useRef } from 'react'
import { GuidanceCard } from '../maps/GuidanceCard'
import { useSplitShown } from '../state/hooks'
import { useCabin } from '../state/store'
import { MapView } from './MapView'
import { useAnyWidget } from './widgetState'
import { DestinationWidget, NowPlayingWidget, WeatherWidget } from './widgets'
import './dashboard.css'

/**
 * CarPlay-style Dashboard: the map takes most of the space (with one ETA pill), and a column of one-job
 * widgets sits on the right: turn-by-turn on top while there's a route, the weather where
 * there's room. In its half of the split screen, no map (the game's own is beside it, owner 2026-11-02: "two maps
 * on one screen make no sense"): the widgets alone, across the half.
 */
export function Dashboard() {
  // No widget to show (those turned on in Settings have nothing, and no route): the map has the whole width.
  const anyWidget = useAnyWidget()
  const split = useSplitShown() !== null
  const useGameMap = useCabin((state) => state.useGameMap)
  return (
    <section className={split ? 'screen dashboard split' : 'screen dashboard'} aria-label="Dashboard">
      {!split && (useGameMap ? <GameMapHole /> : <MapView />)}
      <div className="widgets" hidden={!anyWidget && !split}>
        <GuidanceCard variant="widget" />
        <NowPlayingWidget />
        <DestinationWidget />
        <WeatherWidget />
      </div>
    </section>
  )
}

/** Transparent rounded cutout in the Dashboard map slot so the game's original GPS map shows through (owner 2026-11-22). */
function GameMapHole() {
  const ref = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const measure = () => {
      const carplay = el.closest('.carplay')
      if (!carplay) return
      const a = el.getBoundingClientRect()
      const b = carplay.getBoundingClientRect()
      if (a.width <= 0 || a.height <= 0) return
      const radius = Number.parseFloat(getComputedStyle(el).borderRadius) || 24
      useCabin.getState().setMapHole({
        x: Math.round(a.left - b.left),
        y: Math.round(a.top - b.top),
        width: Math.round(a.width),
        height: Math.round(a.height),
        radius: Math.round(radius),
      })
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    window.addEventListener('resize', measure)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', measure)
      useCabin.getState().setMapHole(null)
    }
  }, [])
  return <div ref={ref} className="map-tile game-map-hole" aria-label="Game GPS" />
}
