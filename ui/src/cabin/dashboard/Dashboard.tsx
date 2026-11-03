import { GuidanceCard } from '../maps/GuidanceCard'
import { useSplitShown } from '../state/hooks'
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
  return (
    <section className={split ? 'screen dashboard split' : 'screen dashboard'} aria-label="Dashboard">
      {!split && <MapView />}
      <div className="widgets" hidden={!anyWidget && !split}>
        <GuidanceCard variant="widget" />
        <NowPlayingWidget />
        <DestinationWidget />
        <WeatherWidget />
      </div>
    </section>
  )
}
