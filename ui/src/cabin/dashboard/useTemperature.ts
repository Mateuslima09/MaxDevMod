import { useMemo } from 'react'
import { cabClock } from '../../shared/host'
import type { MapLabel } from '../maps/mapData'
import { useLive } from '../state/store'
import { placedClimates, temperatureAt } from './temperature'

/**
 * The temperature where the truck is now, °C (temperature.ts: an estimate, the game having none); `null` without the
 * game's weather or the truck. Drawn again only as the rounded degree changes, not at each of the game's frames.
 */
export function useTemperature(labels: readonly MapLabel[] | undefined): number | null {
  const climates = useMemo(() => placedClimates(labels ?? []), [labels])
  return useLive((state) => {
    const t = state.telemetry
    if (!t || !state.weather) return null
    return temperatureAt([t.x, t.z], climates, cabClock(t, state.clockShift), state.weather)
  })
}
