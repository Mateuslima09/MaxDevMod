import type { FleetGarage, FleetTrailer, FleetTruck } from '../shared/host'

function normalize(input: string): string {
  return input
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
}

function hasAny(text: string, words: readonly string[]): boolean {
  return words.some((w) => text.includes(w))
}

export interface FleetVisual {
  photo: string
  badge: string
  accent: string
}

const AMERICAN_BRANDS = [
  'kenworth',
  'peterbilt',
  'freightliner',
  'western star',
  'westernstar',
  'international',
  'mack',
  'volvo vnl',
  'volvo vnr',
]

/** Brand accent color for trucks so every card & details hero has a distinct manufacturer identity. */
function brandAccent(brand: string): string {
  const b = normalize(brand)
  if (b.includes('scania')) return '#0a84ff'
  if (b.includes('volvo')) return '#2563eb'
  if (b.includes('daf')) return '#e53935'
  if (b.includes('mercedes') || b.includes('actros')) return '#94a3b8'
  if (b.includes('man')) return '#d97706'
  if (b.includes('iveco')) return '#0284c7'
  if (b.includes('renault')) return '#eab308'
  if (b.includes('kenworth') || b.includes('peterbilt')) return '#dc2626'
  if (b.includes('freightliner') || b.includes('western')) return '#6366f1'
  if (b.includes('mack') || b.includes('international')) return '#f59e0b'
  if (b.includes('ford') || b.includes('vw') || b.includes('volkswagen')) return '#0ea5e9'
  return '#e53935'
}

/** Maps any owned truck (ETS2, ATS, or BR/EU mods) to a themed vehicle photo + brand accent. */
export function fleetTruckVisual(truck: Pick<FleetTruck, 'brand' | 'model' | 'chassis'>): FleetVisual {
  const text = normalize(`${truck.brand} ${truck.model}`)
  const isAmerican = hasAny(text, AMERICAN_BRANDS)
  const photo = isAmerican ? '/fleet/truck_american.jpg' : '/fleet/truck_euro.jpg'
  const badge = (truck.brand || 'Truck').toUpperCase()
  return {
    photo,
    badge,
    accent: brandAccent(`${truck.brand} ${truck.model}`),
  }
}

/**
 * Maps any owned trailer (graneleiro, carga seca, carrega-tudo, tanque, baú frigorífico, sider, etc.)
 * to a dedicated photorealistic semi-trailer photo under `ui/public/fleet/trailer_*.jpg`.
 */
export function fleetTrailerVisual(
  trailer: Pick<FleetTrailer, 'brand' | 'model' | 'body' | 'bodyType' | 'chainType'>,
  language: 'en' | 'pt',
): FleetVisual {
  const pt = language === 'pt'
  const text = normalize(`${trailer.brand} ${trailer.model} ${trailer.body} ${trailer.bodyType} ${trailer.chainType}`)

  if (hasAny(text, ['tanque', 'tank', 'silo', 'cistern', 'fuel', 'combust', 'foodtank', 'chem'])) {
    return { photo: '/fleet/trailer_tanque.jpg', badge: pt ? 'TANQUE' : 'TANKER', accent: '#f59e0b' }
  }
  if (hasAny(text, ['granel', 'grain', 'hopper', 'bulk', 'caçamba', 'cacamba', 'dumper', 'tipper', 'bascul', 'carga seca', 'manos', 'noma'])) {
    return { photo: '/fleet/trailer_graneleiro.jpg', badge: pt ? 'GRANELEIRO' : 'BULK', accent: '#d97706' }
  }
  if (hasAny(text, ['carrega', 'prancha', 'lowbed', 'lowloader', 'lowboy', 'heavy', 'machin', 'flatbed', 'facchini', 'log', 'timber', 'steel', 'cegonha'])) {
    return { photo: '/fleet/trailer_prancha.jpg', badge: pt ? 'PRANCHA' : 'LOWBED', accent: '#eab308' }
  }
  if (hasAny(text, ['frig', 'reefer', 'refriger', 'insul', 'termic', 'baú', 'bau', 'cold', 'dry', 'sider', 'curtain', 'box', 'container', 'gooseneck'])) {
    return { photo: '/fleet/trailer_bau.jpg', badge: pt ? 'BAÚ / SIDER' : 'BOX / REEFER', accent: '#38bdf8' }
  }
  return { photo: '/fleet/trailer_graneleiro.jpg', badge: pt ? 'CARRETA' : 'TRAILER', accent: '#3b82f6' }
}

/** Maps an owned garage to the flagship garage photo and tier accent. */
export function fleetGarageVisual(garage: Pick<FleetGarage, 'tier' | 'slots'>, language: 'en' | 'pt'): FleetVisual {
  const pt = language === 'pt'
  const badge =
    garage.tier === 'large'
      ? pt
        ? 'GARAGEM GRANDE'
        : 'LARGE GARAGE'
      : garage.tier === 'small'
        ? pt
          ? 'GARAGEM MÉDIA'
          : 'SMALL GARAGE'
        : pt
          ? 'GARAGEM INICIAL'
          : 'TINY GARAGE'
  const accent = garage.tier === 'large' ? '#30d158' : garage.tier === 'small' ? '#0a84ff' : '#f59e0b'
  return {
    photo: '/fleet/garage_hq.jpg',
    badge,
    accent,
  }
}
