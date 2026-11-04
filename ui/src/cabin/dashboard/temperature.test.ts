import { describe, expect, it } from 'vitest'
import type { MapLabel } from '../maps/mapData'
import { fromLngLat } from '../maps/projection'
import { formatTemperature, placedClimates, temperatureAt } from './temperature'

const clear = { now: 'clear', coming: 'clear' } as const
const rain = { now: 'rain', coming: 'rain' } as const
const at = (hour: number) => hour * 60

function city(name: string, lng: number, lat: number, population = 1000, localized?: string): MapLabel {
  return { name, localized, kind: 'city', at: [lng, lat], population }
}

describe('temperatureAt', () => {
  // Phoenix (24 °C a year, a 13 °C day) and Flagstaff (8, 17), 2° apart on the map.
  const climates = placedClimates([city('Phoenix', 0, 0), city('Flagstaff', 0, 2), city('Nowhere', 5, 5)])
  const phoenix = fromLngLat(0, 0)
  const flagstaff = fromLngLat(0, 2)

  it("is a city's own in it, warmest in the afternoon and coldest before dawn", () => {
    // (Hours off a half degree's rounding: 24 + 6.5 cos(15°) is 30.3, 24 − 6.5 cos(15°) 17.7.)
    expect(temperatureAt(phoenix, climates, at(14), clear)).toBe(30)
    expect(temperatureAt(phoenix, climates, at(4), clear)).toBe(18)
    expect(temperatureAt(flagstaff, climates, at(14), clear)).toBe(16)
    // Warmest at 15:00, coldest at 3:00 (whole degrees: as warm as the hours either side, at least).
    const t = (hour: number) => temperatureAt(phoenix, climates, at(hour), clear)
    expect(t(15)).toBeGreaterThanOrEqual(Math.max(t(14), t(16)))
    expect(t(3)).toBeLessThanOrEqual(Math.min(t(2), t(4)))
    expect(t(15) - t(3)).toBe(13)
  })

  it('is weighed between the cities around, the nearer the more', () => {
    const between = temperatureAt(fromLngLat(0, 1), climates, at(9), clear)
    expect(between).toBe(temperatureAt(fromLngLat(0, 1), climates, at(21), clear))
    const nearPhoenix = temperatureAt(fromLngLat(0, 0.3), climates, at(9), clear)
    expect(nearPhoenix).toBeGreaterThan(between)
  })

  it('is cooler and steadier in the rain, and half so while it turns', () => {
    const afternoon = temperatureAt(phoenix, climates, at(15), clear)
    expect(temperatureAt(phoenix, climates, at(15), rain)).toBe(24 + 3 - 3)
    const turning = temperatureAt(phoenix, climates, at(15), { now: 'clear', coming: 'rain' })
    expect(turning).toBeLessThan(afternoon)
    expect(turning).toBeGreaterThan(temperatureAt(phoenix, climates, at(15), rain))
  })

  it("takes a temperate place's without any known city", () => {
    expect(temperatureAt(phoenix, [], at(15), clear)).toBe(19)
    expect(temperatureAt(phoenix, [], at(3), clear)).toBe(9)
  })
})

describe('placedClimates', () => {
  it('knows cities by their own name or the localized one, accents or not, each once', () => {
    const found = placedClimates([
      city('München', 1, 1),
      city('Αθήνα', 2, 2),
      city('Bucuresti', 3, 3),
      city('Wien', 4, 4, 10),
      city('Wien', 9, 9, 99),
      city('Dorf', 5, 5, 10, 'Munich'),
      { ...city('Oslo', 6, 6), kind: 'village' },
    ])
    expect(found.map((climate) => climate.at)).toEqual([
      [1, 1],
      [2, 2],
      [3, 3],
      [9, 9],
    ])
  })
})

describe('formatTemperature', () => {
  it('says °C, or °F with miles', () => {
    expect(formatTemperature(21, 'km')).toBe('21°C')
    expect(formatTemperature(21, 'mi')).toBe('70°F')
    expect(formatTemperature(-3, 'mi')).toBe('27°F')
  })
})
