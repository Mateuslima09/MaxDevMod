import type { Sky } from '../../shared/host'
import type { MapLabel } from '../maps/mapData'
import { metresBetween, toLngLat } from '../maps/projection'
import type { Point } from '../maps/route'

/**
 * The temperature where the truck is, for the weather widgets (owner 2026-11-03: "the temperature in the weather
 * widget"). The game has none (weather.ts), so it's estimated from what the game does have: where the truck is,
 * among cities whose real climate is known (their yearly mean and how far a day swings from it, roughly; the
 * game's own cities, as its map names them), the time of day in the game, and its rain. An estimate in keeping
 * with the game's sky, not a measurement: the same place, hour and sky always give the same.
 */

/** A city whose climate is known: its names (as maps name it), its yearly mean and its day's swing, °C. */
interface Climate {
  names: string[]
  mean: number
  swing: number
}

/** American Truck Simulator's cities and Euro Truck Simulator 2's, a spread of each game's climates. */
const CLIMATES: Climate[] = [
  // The west coast, mild by the sea, hot inland.
  { names: ['Los Angeles'], mean: 18.5, swing: 8 },
  { names: ['San Diego'], mean: 18, swing: 7 },
  { names: ['San Francisco'], mean: 14.5, swing: 7 },
  { names: ['Sacramento'], mean: 16.5, swing: 14 },
  { names: ['Fresno'], mean: 18, swing: 16 },
  { names: ['Bakersfield'], mean: 19, swing: 15 },
  { names: ['Redding'], mean: 17, swing: 17 },
  { names: ['Eureka'], mean: 11.5, swing: 7 },
  { names: ['Medford'], mean: 12.5, swing: 16 },
  { names: ['Portland'], mean: 12.5, swing: 9 },
  { names: ['Seattle'], mean: 11.5, swing: 8 },
  { names: ['Spokane'], mean: 9, swing: 12 },
  // The desert and the mountains.
  { names: ['Las Vegas'], mean: 20.5, swing: 13 },
  { names: ['Reno'], mean: 11, swing: 17 },
  { names: ['Elko'], mean: 8, swing: 19 },
  { names: ['Phoenix'], mean: 24, swing: 13 },
  { names: ['Tucson'], mean: 21, swing: 15 },
  { names: ['Flagstaff'], mean: 8, swing: 17 },
  { names: ['Albuquerque'], mean: 14, swing: 15 },
  { names: ['El Paso'], mean: 18, swing: 14 },
  { names: ['Salt Lake City'], mean: 11.5, swing: 13 },
  { names: ['Boise'], mean: 11, swing: 14 },
  { names: ['Denver'], mean: 10, swing: 15 },
  { names: ['Grand Junction'], mean: 11.5, swing: 15 },
  { names: ['Cheyenne'], mean: 7.5, swing: 13 },
  { names: ['Casper'], mean: 7.5, swing: 15 },
  { names: ['Billings'], mean: 8.5, swing: 13 },
  { names: ['Helena'], mean: 7, swing: 14 },
  // The plains and the south.
  { names: ['Dallas'], mean: 19.5, swing: 11 },
  { names: ['Houston'], mean: 21, swing: 9 },
  { names: ['San Antonio'], mean: 21, swing: 11 },
  { names: ['Austin'], mean: 20.5, swing: 11 },
  { names: ['Amarillo'], mean: 14, swing: 14 },
  { names: ['Oklahoma City'], mean: 16, swing: 12 },
  { names: ['Wichita'], mean: 13.5, swing: 12 },
  { names: ['Kansas City'], mean: 13, swing: 11 },
  { names: ['Omaha'], mean: 11, swing: 11 },
  { names: ['Little Rock'], mean: 17, swing: 11 },
  { names: ['Shreveport'], mean: 18.5, swing: 11 },
  { names: ['New Orleans'], mean: 21, swing: 8 },
  { names: ['Baton Rouge'], mean: 20, swing: 10 },
  { names: ['Memphis'], mean: 17, swing: 10 },
  { names: ['St. Louis', 'Saint Louis'], mean: 14, swing: 10 },
  { names: ['Des Moines'], mean: 10.5, swing: 10 },
  { names: ['Minneapolis'], mean: 8, swing: 10 },
  { names: ['Sioux Falls'], mean: 8, swing: 12 },
  { names: ['Rapid City'], mean: 8.5, swing: 14 },
  { names: ['Bismarck'], mean: 6, swing: 13 },
  { names: ['Fargo'], mean: 5.5, swing: 11 },
  // Europe's west and south.
  { names: ['London'], mean: 11.5, swing: 7 },
  { names: ['Manchester'], mean: 10, swing: 6 },
  { names: ['Glasgow'], mean: 9.5, swing: 6 },
  { names: ['Aberdeen'], mean: 8.5, swing: 6 },
  { names: ['Paris'], mean: 12.5, swing: 8 },
  { names: ['Lyon'], mean: 12.5, swing: 9 },
  { names: ['Bordeaux'], mean: 13.5, swing: 9 },
  { names: ['Marseille'], mean: 15.5, swing: 9 },
  { names: ['Brest'], mean: 11.5, swing: 6 },
  { names: ['Madrid'], mean: 15, swing: 12 },
  { names: ['Sevilla', 'Seville'], mean: 19, swing: 13 },
  { names: ['Valencia'], mean: 18, swing: 8 },
  { names: ['Barcelona'], mean: 16.5, swing: 7 },
  { names: ['Bilbao'], mean: 14.5, swing: 7 },
  { names: ['Lisboa', 'Lisbon'], mean: 17.5, swing: 8 },
  { names: ['Porto'], mean: 15, swing: 8 },
  { names: ['Roma', 'Rome'], mean: 15.5, swing: 10 },
  { names: ['Milano', 'Milan'], mean: 13, swing: 9 },
  { names: ['Napoli', 'Naples'], mean: 16, swing: 8 },
  { names: ['Palermo'], mean: 18.5, swing: 7 },
  { names: ['Athína', 'Αθήνα', 'Athens'], mean: 18.5, swing: 9 },
  { names: ['Thessaloníki', 'Θεσσαλονίκη', 'Thessaloniki'], mean: 16, swing: 10 },
  { names: ['İstanbul', 'Istanbul'], mean: 14.5, swing: 7 },
  // Europe's middle and east.
  { names: ['Amsterdam'], mean: 10.5, swing: 7 },
  { names: ['Brussel', 'Bruxelles', 'Brussels'], mean: 11, swing: 7 },
  { names: ['Luxembourg'], mean: 9.5, swing: 7 },
  { names: ['Berlin'], mean: 10, swing: 8 },
  { names: ['Hamburg'], mean: 9.5, swing: 7 },
  { names: ['Frankfurt am Main', 'Frankfurt'], mean: 10.5, swing: 8 },
  { names: ['München', 'Munich'], mean: 9, swing: 9 },
  { names: ['Zürich', 'Zurich'], mean: 9.5, swing: 8 },
  { names: ['Wien', 'Vienna'], mean: 11, swing: 8 },
  { names: ['Praha', 'Prague'], mean: 9, swing: 8 },
  { names: ['Warszawa', 'Warsaw'], mean: 8.5, swing: 8 },
  { names: ['Kraków', 'Krakow'], mean: 8.5, swing: 9 },
  { names: ['Gdańsk', 'Gdansk'], mean: 8, swing: 7 },
  { names: ['Budapest'], mean: 11.5, swing: 9 },
  { names: ['București', 'Bucureşti', 'Bucharest'], mean: 11, swing: 11 },
  { names: ['Sofia', 'София'], mean: 10.5, swing: 11 },
  // The north.
  { names: ['København', 'Copenhagen'], mean: 9, swing: 6 },
  { names: ['Aarhus'], mean: 8.5, swing: 6 },
  { names: ['Oslo'], mean: 6, swing: 7 },
  { names: ['Bergen'], mean: 8, swing: 5 },
  { names: ['Stockholm'], mean: 7.5, swing: 6 },
  { names: ['Göteborg', 'Gothenburg'], mean: 8, swing: 6 },
  { names: ['Malmö', 'Malmo'], mean: 8.5, swing: 6 },
  { names: ['Helsinki'], mean: 6, swing: 6 },
  { names: ['Turku'], mean: 6, swing: 7 },
  { names: ['Oulu'], mean: 3, swing: 7 },
  { names: ['Tallinn'], mean: 6, swing: 6 },
  { names: ['Rīga', 'Riga'], mean: 7, swing: 7 },
  { names: ['Vilnius'], mean: 7, swing: 8 },
  { names: ['Санкт-Петербург', 'Sankt-Peterburg', 'Saint Petersburg'], mean: 6, swing: 6 },
]

/** Without any known city on the map (a map mod's): a temperate place's. */
const ELSEWHERE: Omit<Climate, 'names'> = { mean: 14, swing: 10 }
/** How many of the nearest known cities the truck's place is weighed between. */
const NEAREST = 3
/** The hour a day is warmest (and twelve hours on, coldest). */
const WARMEST_HOUR = 15
/** How much cooler rain is, °C, and how much of the day's swing it leaves. */
const RAIN_COOLS = 3
const RAIN_SWING = 0.5

/** A known city on this map: where it is (lng, lat) and its climate. */
export interface PlacedClimate {
  at: [number, number]
  mean: number
  swing: number
}

/** A name as it's matched: lower case, without accents, letters and digits only. */
function key(name: string): string {
  return name
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLocaleLowerCase('en')
    .replace(/[^\p{L}\p{N}]/gu, '')
}

const BY_NAME = new Map(CLIMATES.flatMap((climate) => climate.names.map((name) => [key(name), climate] as const)))

/** The map's cities whose climate is known, each once (its biggest place by that name). */
export function placedClimates(labels: readonly MapLabel[]): PlacedClimate[] {
  const found = new Map<Climate, MapLabel>()
  for (const label of labels) {
    if (label.kind !== 'city') continue
    const climate = BY_NAME.get(key(label.name)) ?? (label.localized ? BY_NAME.get(key(label.localized)) : undefined)
    if (!climate) continue
    const before = found.get(climate)
    if (!before || label.population > before.population) found.set(climate, label)
  }
  return [...found].map(([climate, label]) => ({ at: label.at, mean: climate.mean, swing: climate.swing }))
}

/**
 * The temperature at `at` (game metres), °C, rounded: its climate weighed between the nearest known cities (the
 * nearer, the more), at the game's time of day (minutes since day 0), cooler and steadier in its rain (`now`
 * raining, or `coming` while it turns: half of it).
 */
export function temperatureAt(
  at: Point,
  climates: readonly PlacedClimate[],
  gameTime: number,
  weather: { now: Sky; coming: Sky },
): number {
  const here = toLngLat(at[0], at[1])
  const nearest = climates
    .map((climate) => ({ climate, distance: metresBetween(here, climate.at) }))
    .sort((a, b) => a.distance - b.distance)
    .slice(0, NEAREST)
  let { mean, swing } = ELSEWHERE
  if (nearest.length > 0) {
    // Inverse distance squared; in a city, its own.
    const weights = nearest.map(({ distance }) => 1 / Math.max(distance, 1) ** 2)
    const total = weights.reduce((sum, weight) => sum + weight, 0)
    mean = nearest.reduce((sum, { climate }, i) => sum + climate.mean * weights[i], 0) / total
    swing = nearest.reduce((sum, { climate }, i) => sum + climate.swing * weights[i], 0) / total
  }
  // Raining and staying so: all of it; turning either way: half.
  const rain = ((weather.now === 'rain' ? 1 : 0) + (weather.coming === 'rain' ? 1 : 0)) / 2
  const hour = (((gameTime % 1440) + 1440) % 1440) / 60
  const day = Math.cos((2 * Math.PI * (hour - WARMEST_HOUR)) / 24)
  const steadier = 1 - (1 - RAIN_SWING) * rain
  return Math.round(mean + (swing / 2) * steadier * day - RAIN_COOLS * rain)
}

/** A temperature (°C) in the units chosen: °C, or °F with miles. */
export function formatTemperature(celsius: number, units: 'km' | 'mi'): string {
  return units === 'mi' ? `${Math.round((celsius * 9) / 5 + 32)}°F` : `${celsius}°C`
}
