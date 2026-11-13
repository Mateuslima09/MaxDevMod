import type { ExpressionSpecification, LayerSpecification, StyleSpecification } from 'maplibre-gl'
import type { Appearance } from '../state/appearance'
import type { MapData } from '../maps/mapData'
import { pixelsPerMetre } from '../maps/projection'
import { MAP_SOURCE } from '../maps/style'

/**
 * How the reversing camera sees the world: the map's own tiles (crates/mapdata), drawn as a camera would show
 * them rather than as a map: a sky over the horizon, the ground and the game's paved areas, roads of asphalt at
 * their real width with their edges painted, and the buildings standing at their height; no names, no route.
 * Night (the screen's dark look: the headlights on) darker, the far off lost in it.
 */

export const OBSTACLES_SOURCE = 'camera-obstacles'
export const GRID_SOURCE = 'camera-grid'

interface CameraPalette {
  sky: string
  horizon: string
  fog: string
  ground: string
  paved: string
  water: string
  asphalt: string
  paint: string
  buildings: string
  others: string
  grid: string
}

const PALETTES: Record<Appearance, CameraPalette> = {
  light: {
    sky: '#8fb8de',
    horizon: '#dce7ee',
    fog: '#cfd8dc',
    ground: '#7d8a5e',
    paved: '#9a9a94',
    water: '#4d7a96',
    asphalt: '#55585c',
    paint: 'rgba(240, 240, 232, 0.85)',
    buildings: '#c9c3b8',
    others: '#b8352f',
    grid: 'rgba(255, 255, 255, 0.22)',
  },
  dark: {
    sky: '#0b1018',
    horizon: '#1b2430',
    fog: '#121820',
    ground: '#262b22',
    paved: '#34363a',
    water: '#101c26',
    asphalt: '#26282c',
    paint: 'rgba(210, 210, 200, 0.6)',
    buildings: '#4a4844',
    others: '#7a2420',
    grid: 'rgba(255, 255, 255, 0.12)',
  },
}

/** Metres at a zoom as pixels, from street level to the camera's own (the line widths' stops). */
const metres = (value: ExpressionSpecification | number): ExpressionSpecification =>
  [
    'interpolate',
    ['exponential', 2],
    ['zoom'],
    ...[12, 24].flatMap((zoom) => [zoom, ['*', value, pixelsPerMetre(zoom)]]),
  ] as ExpressionSpecification

const WIDTH: ExpressionSpecification = ['to-number', ['get', 'width'], 8]
/** Each road edge's painted line: this wide, this far in from the edge (m). */
const EDGE_LINE = 0.15
const EDGE_IN = 0.35

export function cameraStyle(appearance: Appearance, data: MapData): StyleSpecification {
  const palette = PALETTES[appearance]
  const roads: ExpressionSpecification = ['all', ['!=', ['get', 'class'], 'rail'], ['!', ['to-boolean', ['get', 'tunnel']]]]
  const edge = (side: 1 | -1): LayerSpecification => ({
    id: `camera-road-edge-${side}`,
    type: 'line',
    source: MAP_SOURCE,
    'source-layer': 'road',
    filter: roads,
    layout: { 'line-join': 'round' },
    paint: {
      'line-color': palette.paint,
      'line-width': metres(EDGE_LINE),
      'line-offset': metres(['*', side, ['-', ['/', WIDTH, 2], EDGE_IN]]),
    },
  })
  return {
    version: 8,
    sky: {
      'sky-color': palette.sky,
      'horizon-color': palette.horizon,
      'fog-color': palette.fog,
      'sky-horizon-blend': 0.6,
      'horizon-fog-blend': 0.7,
      'fog-ground-blend': 0.75,
      'atmosphere-blend': 0,
    },
    light: { anchor: 'map', position: [1.3, 210, 35], intensity: appearance === 'dark' ? 0.25 : 0.45 },
    sources: {
      [MAP_SOURCE]: {
        type: 'vector',
        tiles: [data.tiles],
        minzoom: data.minzoom,
        maxzoom: data.maxzoom,
        bounds: data.bounds,
      },
      [OBSTACLES_SOURCE]: { type: 'geojson', data: { type: 'FeatureCollection', features: [] } },
      [GRID_SOURCE]: { type: 'geojson', data: { type: 'FeatureCollection', features: [] } },
    },
    layers: [
      { id: 'camera-ground', type: 'background', paint: { 'background-color': palette.ground } },
      {
        id: 'camera-sea',
        type: 'fill',
        source: MAP_SOURCE,
        'source-layer': 'sea',
        paint: { 'fill-color': palette.water },
      },
      {
        id: 'camera-land',
        type: 'fill',
        source: MAP_SOURCE,
        'source-layer': 'land',
        paint: { 'fill-color': palette.ground },
      },
      {
        id: 'camera-water',
        type: 'fill',
        source: MAP_SOURCE,
        'source-layer': 'water',
        paint: { 'fill-color': palette.water },
      },
      // The game's map areas: its yards, car parks and forecourts, paved.
      {
        id: 'camera-areas',
        type: 'fill',
        source: MAP_SOURCE,
        'source-layer': 'area',
        paint: { 'fill-color': palette.paved },
      },
      { id: 'camera-grid', type: 'line', source: GRID_SOURCE, paint: { 'line-color': palette.grid, 'line-width': 1 } },
      {
        id: 'camera-roads',
        type: 'line',
        source: MAP_SOURCE,
        'source-layer': 'road',
        filter: roads,
        layout: { 'line-join': 'round', 'line-cap': 'round' },
        paint: { 'line-color': palette.asphalt, 'line-width': metres(WIDTH) },
      },
      edge(-1),
      edge(1),
      {
        id: 'camera-buildings',
        type: 'fill-extrusion',
        source: MAP_SOURCE,
        'source-layer': 'building',
        paint: {
          'fill-extrusion-color': palette.buildings,
          'fill-extrusion-height': ['get', 'height'],
          'fill-extrusion-base': ['get', 'base'],
          'fill-extrusion-vertical-gradient': true,
        },
      },
      // Other players' trucks (TruckersMP), as blocks where they are.
      {
        id: 'camera-others',
        type: 'fill-extrusion',
        source: OBSTACLES_SOURCE,
        paint: {
          'fill-extrusion-color': palette.others,
          'fill-extrusion-height': ['get', 'height'],
          'fill-extrusion-base': 0,
        },
      },
    ],
  }
}
