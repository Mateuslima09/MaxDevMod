import maplibregl, { type GeoJSONSource, type GeoJSONSourceSpecification } from 'maplibre-gl'
import type { Appearance } from '../state/appearance'
import { BLANK_TILES } from './mapData'
import { toLngLat } from './projection'

/**
 * The map without a map of the player's (mapData.ts `NO_MAP`, owner 2026-11-02): its tiles all empty, and over its
 * plain ground a grid that moves as the truck does (so the map is seen to move, the truck's speed with it) and the
 * way the truck came, under the game's GPS route (liveMap.ts draws that as on any map).
 */

/** The grid's spacing, how far it reaches round where it was last drawn from, and how far the truck goes before
 * it's drawn again round it, metres. */
const GRID_STEP = 250
const GRID_REACH = 4000
const GRID_REDRAW = 1500
/** The trail: a point every this far, at most this many kept (the oldest let go), and a jump this far (a ferry,
 * a train, another save) starts it anew. */
const TRAIL_STEP = 8
const TRAIL_KEPT = 4000
const TRAIL_JUMP = 500

const GRID_SOURCE = 'blank-grid'
const TRAIL_SOURCE = 'blank-trail'
/** The style's layer they go under: all but its plain ground (style.ts). */
const UNDER = 'beyond'

const PAINT: Record<Appearance, { grid: string; trail: string }> = {
  dark: { grid: 'rgba(255, 255, 255, 0.08)', trail: 'rgba(235, 235, 245, 0.35)' },
  light: { grid: 'rgba(0, 0, 0, 0.08)', trail: 'rgba(60, 60, 67, 0.3)' },
}

let answering = false

/** The empty tiles, answered here (once). */
export function answerBlankTiles(): void {
  if (answering) return
  answering = true
  maplibregl.addProtocol(BLANK_TILES, async () => ({ data: new ArrayBuffer(0) }))
}

/** The way the truck came, as runs of points (`[x, z]` metres), the newest last: kept across maps and screens. */
const trail: [number, number][][] = []
let trailCount = 0

/** Adds where the truck is now to the trail; whether it changed. */
function follow(x: number, z: number): boolean {
  const run = trail.at(-1)
  const last = run?.at(-1)
  const moved = last ? Math.hypot(x - last[0], z - last[1]) : Infinity
  if (last && moved < TRAIL_STEP) return false
  if (!run || moved > TRAIL_JUMP) trail.push([[x, z]])
  else run.push([x, z])
  trailCount++
  while (trailCount > TRAIL_KEPT) {
    trail[0].shift()
    trailCount--
    if (trail[0].length === 0) trail.shift()
  }
  return true
}

/** The lines of the grid round `[x, z]` (metres), on whole steps. */
function gridAround(x: number, z: number): GeoJSONSourceSpecification['data'] {
  const [cx, cz] = [Math.round(x / GRID_STEP) * GRID_STEP, Math.round(z / GRID_STEP) * GRID_STEP]
  const lines: [number, number][][] = []
  for (let offset = -GRID_REACH; offset <= GRID_REACH; offset += GRID_STEP) {
    lines.push([toLngLat(cx + offset, cz - GRID_REACH), toLngLat(cx + offset, cz + GRID_REACH)])
    lines.push([toLngLat(cx - GRID_REACH, cz + offset), toLngLat(cx + GRID_REACH, cz + offset)])
  }
  return { type: 'Feature', properties: {}, geometry: { type: 'MultiLineString', coordinates: lines } }
}

function trailLine(): GeoJSONSourceSpecification['data'] {
  const runs = trail.filter((run) => run.length > 1).map((run) => run.map(([x, z]) => toLngLat(x, z)))
  return { type: 'Feature', properties: {}, geometry: { type: 'MultiLineString', coordinates: runs } }
}

/** What a map's grid was last drawn round, and in which appearance it's painted. */
const drawn = new WeakMap<maplibregl.Map, { grid: [number, number] | null; painted: Appearance }>()

/**
 * Each frame of a map without a map: its grid and trail added once its style has loaded, the trail following the
 * truck (`truck`, `null` without one), the grid drawn again round it as it goes on, both in `appearance`.
 */
export function syncBlank(map: maplibregl.Map, appearance: Appearance, truck: { x: number; z: number } | null) {
  const changed = truck ? follow(truck.x, truck.z) : false
  let state = drawn.get(map)
  if (!state) {
    if (!map.isStyleLoaded() || !map.getLayer(UNDER)) return
    map.addSource(GRID_SOURCE, { type: 'geojson', data: { type: 'FeatureCollection', features: [] } })
    map.addSource(TRAIL_SOURCE, { type: 'geojson', data: trailLine() })
    const paint = PAINT[appearance]
    map.addLayer(
      { id: GRID_SOURCE, type: 'line', source: GRID_SOURCE, paint: { 'line-color': paint.grid, 'line-width': 1 } },
      UNDER,
    )
    map.addLayer(
      {
        id: TRAIL_SOURCE,
        type: 'line',
        source: TRAIL_SOURCE,
        layout: { 'line-cap': 'round', 'line-join': 'round' },
        paint: {
          'line-color': paint.trail,
          'line-width': ['interpolate', ['exponential', 2], ['zoom'], 10, 2, 16, 6, 19, 14],
        },
      },
      UNDER,
    )
    state = { grid: null, painted: appearance }
    drawn.set(map, state)
  }
  if (state.painted !== appearance) {
    state.painted = appearance
    map.setPaintProperty(GRID_SOURCE, 'line-color', PAINT[appearance].grid)
    map.setPaintProperty(TRAIL_SOURCE, 'line-color', PAINT[appearance].trail)
  }
  if (changed) map.getSource<GeoJSONSource>(TRAIL_SOURCE)?.setData(trailLine())
  if (truck && (!state.grid || Math.hypot(truck.x - state.grid[0], truck.z - state.grid[1]) > GRID_REDRAW)) {
    state.grid = [truck.x, truck.z]
    map.getSource<GeoJSONSource>(GRID_SOURCE)?.setData(gridAround(truck.x, truck.z))
  }
}
