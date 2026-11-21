import maplibregl, { type GeoJSONSource } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import type { Telemetry } from '../../shared/host'
import { answerBlankTiles } from '../maps/blankMap'
import { mapFrameDue } from '../maps/frameCap'
import type { MapData } from '../maps/mapData'
import { fromLngLat, toLngLat } from '../maps/projection'
import { MAP_SOURCE } from '../maps/style'
import type { Appearance } from '../state/appearance'
import { useLive } from '../state/store'
import { cameraStyle, GRID_SOURCE, OBSTACLES_SOURCE } from './cameraStyle'
import {
  backOf,
  bearingOf,
  cameraOf,
  type PathPoint,
  type Rig,
  rearOf,
  reversePath,
  rightOf,
  rigOf,
  type Vec,
  wrap,
} from './geometry'
import { GUIDE_MARKS, pointAt, quad, towards } from './guides'
import { type Obstacle, readSensors, withRays } from './sensor'

/**
 * The reversing camera's picture: a map of its own (not the Maps app's: that one goes on following the truck
 * from above, held still under this), its camera on the rig's rear looking back and down, as a real reversing
 * camera does, and its picture mirrored, as theirs is (the left of the screen is the driver's left). Over it,
 * drawn on the ground in perspective, the lines where the rear goes with the wheels as they are, marked at a
 * metre, two and three, which bend as the wheel is turned; and each frame the sensors' readings, for the screen
 * round it and the beeps.
 *
 * It follows the telemetry a frame behind (between its last two samples), so it moves smoothly though the
 * telemetry comes no more often than the frames do.
 */

/** The camera's tilt down from the horizon (degrees) and its picture's height (degrees: a wide lens). On a
 * trailer, steeper, to see the ground right under its doors. */
const TRAILER_TILT = 42
const TRUCK_TILT = 36
const VERTICAL_FOV = 78
/** How far ahead of the rear end the camera sits (m): on the trailer's frame, over the doors. */
const CAMERA_IN = 0.15
/** How far the guide lines reach behind the rear (m), and their marks. */
const GUIDE_LENGTH = 6
/** How wide the guide lines are painted on the ground, metres. */
const GUIDE_WIDTH = 0.12
/** How often the obstacles are read again from the map's tiles (ms), and how far round the rear (m). */
const OBSTACLES_EVERY_MS = 400
const OBSTACLES_REACH = 30
/** Others' trucks (TruckersMP): their size as blocks (m). */
const OTHER_HALF_WIDTH = 1.3
const OTHER_HALF_LENGTH = 4
const OTHER_HEIGHT = 3.6
/** Buildings standing higher than this off the ground (a canopy's roof) are over the rig, not in its way. */
const OVERHEAD = 4.4
/** The no-map grid's spacing and reach round the camera, and how far it moves before it's drawn again (m). */
const GRID_STEP = 2
const GRID_REACH = 60
const GRID_REDRAW = 20

/** What each frame tells the screen round the picture. */
export interface Frame {
  /** Each sensor's reading (sensor.ts SENSORS; `null`: nothing in range). */
  readings: (number | null)[]
  /** The trailer coupled (the camera on it), and its angle to the truck (radians). */
  trailer: boolean
  articulation: number
}

export interface CameraView {
  element: HTMLDivElement
  /** Shows the picture into `host`, `appearance` its look, and calls `onFrame` with each frame drawn. */
  show(host: HTMLElement, data: MapData, appearance: Appearance, onFrame: (frame: Frame) => void): void
  hide(): void
  /** Obstacles of the page's own (the browser preview's), besides the map's. */
  extraObstacles: Obstacle[]
  /** Whether the guide lines are drawn (Settings). */
  guides: boolean
}

interface Sample {
  at: number
  telemetry: Telemetry
}

/** The telemetry between two samples, `k` of the way from the first (positions, headings and the steering). */
function between(a: Telemetry, b: Telemetry, k: number): Telemetry {
  const lerp = (x: number, y: number) => x + (y - x) * k
  // Headings in turns: the short way round.
  const turns = (x: number, y: number) => x + (wrap((y - x) * 2 * Math.PI) / (2 * Math.PI)) * k
  return {
    ...b,
    x: lerp(a.x, b.x),
    z: lerp(a.z, b.z),
    heading: turns(a.heading, b.heading),
    wheelSteering: lerp(a.wheelSteering, b.wheelSteering),
    trailerHeading: turns(a.trailerHeading, b.trailerHeading),
  }
}

let view: CameraView | null = null

/** The camera's picture: made once, kept for the next time the truck reverses. */
export function cameraView(): CameraView {
  view ??= makeView()
  return view
}

function makeView(): CameraView {
  answerBlankTiles()
  const element = document.createElement('div')
  element.className = 'reversing-picture'
  const mapElement = document.createElement('div')
  mapElement.className = 'reversing-map'
  const overlay = document.createElement('canvas')
  overlay.className = 'reversing-guides'
  element.append(mapElement, overlay)

  let map: maplibregl.Map | null = null
  let shownData: MapData | null = null
  let shownAppearance: Appearance | null = null
  let frame: number | null = null
  let onFrame: (frame: Frame) => void = () => {}
  const samples: Sample[] = []
  let obstacles: Obstacle[] = []
  let obstaclesAt = -Infinity
  let gridFrom: Vec | null = null
  let path: PathPoint[] = []
  let rig: Rig | null = null

  const result: CameraView = {
    element,
    extraObstacles: [],
    guides: true,
    show(host, data, appearance, frameCallback) {
      onFrame = frameCallback
      if (element.parentElement !== host) host.appendChild(element)
      if (!map || shownData !== data) {
        map?.remove()
        map = makeMap(data, appearance)
        shownData = data
        shownAppearance = appearance
        gridFrom = null
      } else if (shownAppearance !== appearance) {
        map.setStyle(cameraStyle(appearance, data))
        shownAppearance = appearance
        gridFrom = null
      }
      map.resize()
      samples.length = 0
      if (frame === null) frame = requestAnimationFrame(step)
    },
    hide() {
      element.remove()
      if (frame !== null) cancelAnimationFrame(frame)
      frame = null
    },
  }

  function makeMap(data: MapData, appearance: Appearance): maplibregl.Map {
    const made = new maplibregl.Map({
      container: mapElement,
      style: cameraStyle(appearance, data),
      center: data.centre,
      zoom: 20,
      maxZoom: 24,
      maxPitch: 89,
      interactive: false,
      attributionControl: false,
      fadeDuration: 0,
      // Its camera where the rig's is, not held to the ground under the picture's middle.
      centerClampedToGround: false,
      canvasContextAttributes: { antialias: true },
    })
    made.setVerticalFieldOfView(VERTICAL_FOV)
    made.on('render', () => drawGuides())
    return made
  }

  let due = 0
  function step(now: number) {
    frame = requestAnimationFrame(step)
    const live = useLive.getState()
    const telemetry = live.telemetry
    if (!telemetry || !map) return
    const last = samples.at(-1)
    if (!last || last.telemetry !== telemetry) {
      samples.push({ at: now, telemetry })
      if (samples.length > 2) samples.shift()
    }
    const next = mapFrameDue(now, due)
    if (next === null) return
    due = next
    // A frame behind: between the last two samples, as far on from the first as the time since the last.
    const [a, b] = samples.length === 2 ? samples : [samples[0], samples[0]]
    const k = b.at > a.at ? Math.min(1, (now - b.at) / (b.at - a.at)) : 1
    rig = rigOf(between(a.telemetry, b.telemetry, k))
    if (!rig) return
    const rear = rearOf(rig)
    if (now - obstaclesAt > OBSTACLES_EVERY_MS) {
      obstaclesAt = now
      obstacles = readObstacles(map, rear.at)
    }
    path = reversePath(rig, GUIDE_LENGTH)
    onFrame({
      readings: withRays(readSensors(rear, [...obstacles, ...result.extraObstacles]), b.telemetry),
      trailer: rig.trailer !== null,
      articulation: rig.trailer ? wrap(rig.trailer.heading - rig.heading) : 0,
    })
    // The picture is sent to the game only while it's seen (crates/host camera.rs Unseen): the sensors go on.
    if (!live.screenSeen) return
    if (shownData?.blank) drawGrid(map, rear.at)
    placeCamera(map, rig)
  }

  function placeCamera(target: maplibregl.Map, rig: Rig) {
    const place = cameraOf(rig)
    const at: Vec = [
      place.at[0] - backOf(rearOf(rig).heading)[0] * CAMERA_IN,
      place.at[1] - backOf(rearOf(rig).heading)[1] * CAMERA_IN,
    ]
    const tilt = rig.trailer ? TRAILER_TILT : TRUCK_TILT
    const options = target.calculateCameraOptionsFromCameraLngLatAltRotation(
      toLngLat(at[0], at[1]),
      place.height,
      bearingOf(place.facing),
      90 - tilt,
      0,
    )
    // MapLibre (5.24) leaves the roll out of its answer when none is given, and jumpTo then makes the camera's
    // matrices of it (NaN): nothing is drawn. Level, said both ways.
    target.jumpTo({ ...options, roll: 0 })
  }

  function drawGrid(target: maplibregl.Map, at: Vec) {
    if (gridFrom && Math.hypot(at[0] - gridFrom[0], at[1] - gridFrom[1]) < GRID_REDRAW) return
    const source = target.getSource<GeoJSONSource>(GRID_SOURCE)
    if (!source) return
    gridFrom = at
    const lines: [number, number][][] = []
    const [x0, z0] = [Math.round(at[0] / GRID_STEP) * GRID_STEP, Math.round(at[1] / GRID_STEP) * GRID_STEP]
    for (let d = -GRID_REACH; d <= GRID_REACH; d += GRID_STEP) {
      lines.push([toLngLat(x0 + d, z0 - GRID_REACH), toLngLat(x0 + d, z0 + GRID_REACH)])
      lines.push([toLngLat(x0 - GRID_REACH, z0 + d), toLngLat(x0 + GRID_REACH, z0 + d)])
    }
    source.setData({ type: 'Feature', properties: {}, geometry: { type: 'MultiLineString', coordinates: lines } })
  }

  function drawGuides() {
    const target = map
    const context = overlay.getContext('2d')
    if (!target || !context) return
    const ratio = window.devicePixelRatio || 1
    const width = overlay.clientWidth
    const height = overlay.clientHeight
    if (overlay.width !== Math.round(width * ratio) || overlay.height !== Math.round(height * ratio)) {
      overlay.width = Math.round(width * ratio)
      overlay.height = Math.round(height * ratio)
    }
    context.setTransform(ratio, 0, 0, ratio, 0, 0)
    context.clearRect(0, 0, width, height)
    if (!rig || path.length < 2 || !result.guides) return
    const project = ([x, z]: Vec) => {
      const point = target.project(toLngLat(x, z))
      return [point.x, point.y] as const
    }
    // Each side's line as a strip on the ground: its outer edge on the rear's corner path, its inner edge a
    // line's width in, coloured by how far back it is.
    const colourAt = (travelled: number) =>
      travelled < GUIDE_MARKS[0] ? '#ff3b30' : travelled < GUIDE_MARKS[1] ? '#ffcc00' : '#34c759'
    context.shadowColor = 'rgba(0, 0, 0, 0.55)'
    context.shadowBlur = 3
    for (const side of ['left', 'right'] as const) {
      const inward = side === 'left' ? 1 : -1
      for (let at = 1; at < path.length; at++) {
        const [a, b] = [path[at - 1], path[at]]
        const inA = towards(a[side], a.middle, GUIDE_WIDTH * inward)
        const inB = towards(b[side], b.middle, GUIDE_WIDTH * inward)
        quad(context, [project(a[side]), project(b[side]), project(inB), project(inA)], colourAt(a.travelled))
      }
    }
    // The marks across, at each metre: a short tick in from each side.
    for (const mark of GUIDE_MARKS) {
      const point = pointAt(path, mark)
      if (!point) continue
      const reach = Math.min(0.6, Math.hypot(point.right[0] - point.left[0], point.right[1] - point.left[1]) / 4)
      for (const side of ['left', 'right'] as const) {
        const end = towards(point[side], point.middle, reach)
        const ahead = pointAt(path, mark + GUIDE_WIDTH)
        if (!ahead) continue
        const endAhead = towards(ahead[side], ahead.middle, reach)
        quad(
          context,
          [project(point[side]), project(end), project(endAhead), project(ahead[side])],
          colourAt(mark - 0.01),
        )
      }
    }
    // The way straight back (the rig as it stands), faintly: where it'd go with the wheels straight.
    context.shadowBlur = 0
    context.setLineDash([10, 8])
    context.strokeStyle = 'rgba(255, 255, 255, 0.35)'
    context.lineWidth = 2
    const rear = rearOf(rig)
    const back = backOf(rear.heading)
    context.beginPath()
    const [sx, sy] = project(rear.at)
    const [ex, ey] = project([rear.at[0] + back[0] * GUIDE_LENGTH, rear.at[1] + back[1] * GUIDE_LENGTH])
    context.moveTo(sx, sy)
    context.lineTo(ex, ey)
    context.stroke()
    context.setLineDash([])
  }

  /** Buildings (their outlines, but those overhead) and other players' trucks round `at`, from the map. */
  function readObstacles(target: maplibregl.Map, at: Vec): Obstacle[] {
    const found: Obstacle[] = []
    if (target.getSource(MAP_SOURCE)) {
      for (const feature of target.querySourceFeatures(MAP_SOURCE, { sourceLayer: 'building' })) {
        const base = Number(feature.properties?.base ?? 0)
        if (base > OVERHEAD) continue
        const geometry = feature.geometry
        const polygons: number[][][][] =
          geometry.type === 'Polygon'
            ? [geometry.coordinates]
            : geometry.type === 'MultiPolygon'
              ? geometry.coordinates
              : []
        for (const polygon of polygons) {
          const ring = polygon[0]?.map(([lng, lat]): Vec => fromLngLat(lng, lat))
          if (!ring || ring.length < 3) continue
          if (!ring.some(([x, z]) => Math.hypot(x - at[0], z - at[1]) < OBSTACLES_REACH * 3)) continue
          found.push({ kind: 'outline', ring })
        }
      }
    }
    const others = othersAround(at)
    for (const other of others) found.push({ kind: 'outline', ring: other })
    const source = target.getSource<GeoJSONSource>(OBSTACLES_SOURCE)
    source?.setData({
      type: 'FeatureCollection',
      features: [...others, ...result.extraObstacles.flatMap(ringOf)].map((ring) => ({
        type: 'Feature',
        properties: { height: OTHER_HEIGHT },
        geometry: { type: 'Polygon', coordinates: [[...ring, ring[0]].map(([x, z]) => toLngLat(x, z))] },
      })),
    })
    return found
  }

  return result
}

/** An obstacle of the page's own as the block it's drawn as. */
function ringOf(obstacle: Obstacle): Vec[][] {
  return obstacle.kind === 'outline' ? [obstacle.ring] : []
}

/** Other players' trucks within reach of `at` (TruckersMP's SDK, round the player), each as its outline on the
 * ground. */
function othersAround(at: Vec): Vec[][] {
  const nearby = useLive.getState().nearby
  if (!nearby) return []
  const outlines: Vec[][] = []
  for (const [id, x, z, turns] of nearby.players) {
    if (id === nearby.local || Math.hypot(x - at[0], z - at[1]) > OBSTACLES_REACH) continue
    const heading = turns * 2 * Math.PI
    const back = backOf(heading)
    const right = rightOf(heading)
    const corner = (along: number, across: number): Vec => [
      x + back[0] * along + right[0] * across,
      z + back[1] * along + right[1] * across,
    ]
    outlines.push([
      corner(-OTHER_HALF_LENGTH, -OTHER_HALF_WIDTH),
      corner(-OTHER_HALF_LENGTH, OTHER_HALF_WIDTH),
      corner(OTHER_HALF_LENGTH, OTHER_HALF_WIDTH),
      corner(OTHER_HALF_LENGTH, -OTHER_HALF_WIDTH),
    ])
  }
  return outlines
}




export { GUIDE_MARKS } from './guides'
