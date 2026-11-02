import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { TruckInfo } from '../../shared/host'
import type { ScreenLayoutId } from './screen'

/**
 * The player's own fit of WestPlay on a vehicle's screen (owner 2026-11-02: "a little menu to adjust the size, if
 * needed"), for screens WestPlay draws on without having been measured in the cab (a car's, a mod truck's): the
 * margins the display cuts off, the proportion the game stretches the page to, and the size of what's on it. Done
 * on the page alone, it shows as it's changed, the game left running.
 */
export interface ScreenFit {
  /** Each edge's margin, a fraction of the page (0: none). */
  top: number
  bottom: number
  left: number
  right: number
  /** How wide the page is drawn for its height (1: as laid out; less: narrower, for a screen stretching it). */
  stretch: number
  /** What's on it, times the screen's own size (1: as it is). */
  size: number
}

export type FitEdge = 'top' | 'bottom' | 'left' | 'right'

export const NO_FIT: ScreenFit = { top: 0, bottom: 0, left: 0, right: 0, stretch: 1, size: 1 }

/** How far each setting goes. */
export const FIT_LIMITS = {
  edge: { min: 0, max: 0.2, step: 0.005 },
  stretch: { min: 0.75, max: 1.35, step: 0.01 },
  size: { min: 0.6, max: 1.6, step: 0.05 },
} as const

/** The fit's key: the screen and the vehicle on it (`null` until a screen is drawn); one layout is in several
 * vehicles, not always shown alike. */
export function fitKey(layout: ScreenLayoutId | null, truck: TruckInfo | null): string | null {
  if (layout === null) return null
  return `${layout}|${truck ? truck.id.replace(/^vehicle\./, '') : ''}`
}

/** Whether a fit changes anything. */
export function isFitted(fit: ScreenFit): boolean {
  return (Object.keys(NO_FIT) as (keyof ScreenFit)[]).some((name) => fit[name] !== NO_FIT[name])
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
          const fit = readFit({ ...(state.fits[key] ?? NO_FIT), ...change })
          const fits = { ...state.fits }
          if (isFitted(fit)) fits[key] = fit
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
      merge: (stored, current) => {
        const kept = (stored as { fits?: Record<string, unknown> } | undefined)?.fits ?? {}
        const fits = Object.fromEntries(Object.entries(kept).map(([key, fit]) => [key, readFit(fit)]))
        return { ...current, fits }
      },
    },
  ),
)
