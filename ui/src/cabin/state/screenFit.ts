import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { isInHost, postToHost, type ScreenFitValues, type TruckInfo } from '../../shared/host'
import type { ScreenLayoutId } from './screen'

/**
 * The player's own fit of WestPlay on a vehicle's screen (owner 2026-11-02: "a little menu to adjust the size, if
 * needed"), for screens WestPlay draws on without having been measured in the cab (a car's, a mod truck's): the
 * margins the display cuts off, the proportion the game stretches the page to, and the size of what's on it. Done
 * on the page alone, it shows as it's changed, the game left running.
 */
/**
 * Each edge's margin, a fraction of the page (0: none); `stretch`, how wide the page is drawn for its height (1: as
 * laid out; less: narrower, for a screen stretching it); `size`, what's on it, times the screen's own size.
 */
export type ScreenFit = ScreenFitValues

export type FitEdge = 'top' | 'bottom' | 'left' | 'right'

export const NO_FIT: ScreenFit = { top: 0, bottom: 0, left: 0, right: 0, stretch: 1, size: 1 }

/**
 * The stock GPS screen's fit to start from (owner 2026-11-02): WestPlay's image is on its whole square, and every
 * truck's display shows its own part of it; the 7" Scania's (its full width but 0.5% each side, rows 12.2% to 73.5%:
 * crates/setup game_mod.rs SHOWN_PARTS) until the player fits another truck's, its edges going either way from here (the
 * Actros 2014's display shows further down).
 */
export const GENERIC_FIT: ScreenFit = { top: 0.122, bottom: 0.265, left: 0.005, right: 0.005, stretch: 1, size: 1 }

/** The fit a screen has before the player's own (`key`, [`fitKey`]'s): the stock screen's, else none. */
export function baseFit(key: string | null): ScreenFit {
  return key?.startsWith('generic|') ? GENERIC_FIT : NO_FIT
}

/** How far each setting goes. */
export const FIT_LIMITS = {
  edge: { min: 0, max: 0.35, step: 0.005 },
  stretch: { min: 0.75, max: 1.35, step: 0.01 },
  size: { min: 0.6, max: 1.6, step: 0.05 },
} as const

/** The fit's key: the screen and the vehicle on it (`null` until a screen is drawn); one layout is in several
 * vehicles, not always shown alike. */
export function fitKey(layout: ScreenLayoutId | null, truck: TruckInfo | null): string | null {
  if (layout === null) return null
  // A mod screen's slot is setup's choice, another as the installed mods change: the vehicle's fit, whichever.
  const screen = layout.startsWith('mod-') ? 'mod' : layout
  return `${screen}|${truck ? truck.id.replace(/^vehicle\./, '') : ''}`
}

/**
 * An edge as the player adjusts it (owner 2026-11-02: "negative values too"): from where the screen starts (`base`,
 * [`baseFit`]), 0 there, negative growing WestPlay past it as far as its image goes (the stock screen's 7" part: the
 * Actros 2014's display shows further down), positive cutting it in. Its limits, and the fit's edge for a value.
 */
export function edgeLimits(base: ScreenFit, edge: FitEdge): { min: number; max: number; step: number } {
  const { max, step } = FIT_LIMITS.edge
  return { min: -base[edge], max: Number((max - base[edge]).toFixed(4)), step }
}

export function edgeFrom(base: ScreenFit, edge: FitEdge, value: number): number {
  return Number((base[edge] + value).toFixed(4))
}

/** Whether a fit changes anything from `base` (the screen's own, [`baseFit`]). */
export function isFitted(fit: ScreenFit, base: ScreenFit = NO_FIT): boolean {
  return (Object.keys(NO_FIT) as (keyof ScreenFit)[]).some((name) => fit[name] !== base[name])
}

/** Where the page's root goes for `fit` (`null`: where it is, the whole page): inside the margins, laid out as wide
 * as the proportion asks and squeezed (or widened) back into them. */
export function fitStyle(fit: ScreenFit): Record<'left' | 'top' | 'width' | 'height' | 'transform', string> | null {
  if (fit.top === 0 && fit.bottom === 0 && fit.left === 0 && fit.right === 0 && fit.stretch === 1) return null
  const percent = (value: number) => `${Number((value * 100).toFixed(3))}`
  return {
    left: `${percent(fit.left)}vw`,
    top: `${percent(fit.top)}vh`,
    width: `${percent((1 - fit.left - fit.right) / fit.stretch)}vw`,
    height: `${percent(1 - fit.top - fit.bottom)}vh`,
    transform: `scaleX(${fit.stretch})`,
  }
}

function clamp(value: number, { min, max }: { min: number; max: number }): number {
  return Math.min(max, Math.max(min, value))
}

/** A fit kept as it was read, each setting within its limits (anything else: none). */
export function readFit(stored: unknown): ScreenFit {
  const fit = { ...NO_FIT }
  if (typeof stored !== 'object' || stored === null) return fit
  const read = stored as Partial<Record<keyof ScreenFit, unknown>>
  for (const edge of ['top', 'bottom', 'left', 'right'] as const) {
    if (typeof read[edge] === 'number') fit[edge] = clamp(read[edge], FIT_LIMITS.edge)
  }
  if (typeof read.stretch === 'number') fit.stretch = clamp(read.stretch, FIT_LIMITS.stretch)
  if (typeof read.size === 'number') fit.size = clamp(read.size, FIT_LIMITS.size)
  return fit
}

interface FitState {
  fits: Record<string, ScreenFit>
  change(key: string, change: Partial<ScreenFit>): void
  reset(key: string): void
}

export const useScreenFits = create<FitState>()(
  persist(
    (set) => ({
      fits: {},
      change: (key, change) =>
        set((state) => {
          const fit = readFit({ ...(state.fits[key] ?? baseFit(key)), ...change })
          const fits = { ...state.fits }
          if (isFitted(fit, baseFit(key))) fits[key] = fit
          else delete fits[key]
          return { fits }
        }),
      reset: (key) =>
        set((state) => {
          const fits = { ...state.fits }
          delete fits[key]
          return { fits }
        }),
    }),
    {
      name: 'westplay.screenFit',
      // 1: the stock screen's fits from the whole of it, not the 7"'s part (those kept before let go).
      version: 1,
      migrate: (stored, version) => {
        const fits = (stored as { fits?: Record<string, unknown> } | undefined)?.fits ?? {}
        if (version >= 1) return { fits }
        return { fits: Object.fromEntries(Object.entries(fits).filter(([key]) => !key.startsWith('generic|'))) }
      },
      merge: (stored, current) => {
        const kept = (stored as { fits?: Record<string, unknown> } | undefined)?.fits ?? {}
        const fits = Object.fromEntries(Object.entries(kept).map(([key, fit]) => [key, readFit(fit)]))
        return { ...current, fits }
      },
    },
  ),
)

/**
 * The fit on the cab's screen as the phones know it (owner 2026-11-02: "and if the screen is so small I can't reach
 * Settings?"): Adjust Screen is on the phone in the game, and the player's own, too; the cab's page tells them.
 */
export const useShownFit = create<{ key: string | null; fit: ScreenFit }>()(() => ({ key: null, fit: NO_FIT }))

/** The cab's page's own fit, for the phones (`screenFitShown`). */
export function announceFit(key: string | null, fit: ScreenFit): void {
  if (isInHost()) postToHost({ type: 'screenFitShown', key, fit })
}

/** A phone's Adjust Screen (`screenFit`), in the cab's page alone (the phones hear it too, and keep none): `key` the
 * cab's screen's; told back, whatever it changed. */
export function applyPhoneFit(key: string | null, change: Partial<ScreenFit> | undefined, reset: boolean): void {
  const fits = useScreenFits.getState()
  if (key !== null && reset) fits.reset(key)
  else if (key !== null && change) fits.change(key, change)
  announceFit(key, key === null ? NO_FIT : (useScreenFits.getState().fits[key] ?? baseFit(key)))
}
