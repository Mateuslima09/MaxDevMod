import { useShallow } from 'zustand/react/shallow'
import { formatGameTime, formatTripDistance, formatTripTime, speedKmh } from '../../shared/format'
import { hvtMapMessage } from '../maps/blockedMap'
import { GuidanceCard } from '../maps/GuidanceCard'
import { DashboardMapControls, MapControls } from '../maps/MapControls'
import { MapSearch } from '../maps/MapSearch'
import { nextStop } from '../maps/nextStop'
import { NO_MAP, useMapData } from '../maps/mapData'
import { RoadMap } from '../maps/RoadMap'
import { useLive } from '../state/store'
import { useText } from '../state/text'
import './map.css'
import { cabClock } from '../../shared/host'

/**
 * The map tile (Dashboard and the Maps app): the live map following the truck (with the game's GPS route),
 * the trip's arrival, time and distance left while there's a route, the road's speed limit and the live
 * speed; in the Maps app, turn-by-turn over the map (the Dashboard has its own widget for it). Without a map
 * of the player's (a protected map mod, or none built yet) the live map goes on without roads: the truck, the way
 * it came and the game's GPS route (mapData.ts NO_MAP). `interactive` (the Maps app): the player can move the
 * map, with its buttons; on the Dashboard, it only zooms (its buttons fading while unused).
 * On a map WestPlay doesn't allow (by HVT), it says so instead, in the game's language.
 */
export function MapView({ interactive = false }: { interactive?: boolean }) {
  const speed = useLive((state) => (state.telemetry ? speedKmh(state.telemetry.speed) : 0))
  const limit = useLive((state) => (state.telemetry ? speedKmh(state.telemetry.navigationSpeedLimit) : 0))
  const data = useMapData()
  const hvtMap = useLive((state) => state.hvtMap)
  const hasRoute = useLive((state) => state.route !== null)
  const text = useText()

  if (hvtMap) {
    return (
      <div className="map-tile map-blocked">
        <svg className="map-blocked-glyph" viewBox="0 0 24 24" aria-hidden>
          <path d="M3.5 6.5 9 4l6 2.5L20.5 4v13.5L15 20l-6-2.5-5.5 2.5Z" />
          <path d="M9 4v13.5M15 6.5V20M3 3l18 18" />
        </svg>
        <p>{hvtMapMessage(hvtMap.language)}</p>
      </div>
    )
  }

  // No map of the player's (a protected map mod, or none built yet): the live map all the same, without roads
  // (mapData.ts NO_MAP), the truck, the way it came and the game's route on it. With a map, that map.
  const shown = data ?? (data === null ? NO_MAP : undefined)

  return (
    <div className="map-tile">
      {shown && <RoadMap data={shown} interactive={interactive} />}
      <div className="map-shade" />
      {shown?.blank && !hasRoute && <div className="no-map-pill glass">{text.noMap}</div>}
      <TripPill />
      <div className="speed-stack">
        {limit > 0 && (
          <div className="speed-limit" aria-label={text.speedLimit(limit)}>
            {limit}
          </div>
        )}
        {speed > 0 && <div className={`speed-badge${limit > 0 && speed > limit ? ' over' : ''}`}>{speed}</div>}
      </div>
      {interactive && <GuidanceCard variant="banner" />}
      {shown && interactive && <MapControls />}
      {shown && !interactive && <DashboardMapControls />}
      {/* No places to search without a map. */}
      {data && interactive && <MapSearch />}
    </div>
  )
}

/**
 * The trip, while the game's GPS has a route: when the truck arrives (on the game's clock), the time and
 * the distance left, as the game itself counts them (its time scale and its map's scale, so the same as
 * its own GPS shows). With a stop of the player's ahead, the time to it first ("30 min | 3 h 5 min"), the
 * next one's only, the one after once it's reached (owner 2026-10-21).
 */
function TripPill() {
  const text = useText()
  const trip = useLive(
    useShallow((state) => {
      const telemetry = state.telemetry
      if (!telemetry || telemetry.navigationDistance <= 0) return null
      const minutes = telemetry.navigationTime / 60
      const stop = state.route
        ? nextStop(
            state.routeStops,
            state.route.distances,
            state.routeNext,
            telemetry.navigationDistance,
            telemetry.navigationTime,
          )
        : null
      const total = formatTripTime(minutes, text)
      return [
        formatGameTime(Math.round(cabClock(telemetry, state.clockShift) + minutes)),
        stop ? `${formatTripTime(stop.minutes, text)} | ${total}` : total,
        formatTripDistance(telemetry.navigationDistance, text.decimal),
      ]
    }),
  )
  if (!trip) return null
  const [arrival, time, distance] = trip
  return (
    <div className="eta-pill glass">
      <b>{arrival}</b> {text.arrival} <span className="dot">·</span> {time} <span className="dot">·</span> {distance}
    </div>
  )
}
