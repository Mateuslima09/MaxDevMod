import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { isInHost, postToHost, type ScreenFitValues, type TruckInfo } from '../../shared/host'
import type { ScreenLayoutId } from './screen'

/**
 * The player's own fit of MaxDevMod on a vehicle's screen (for unmapped mod vehicles or custom tuning):
 * margins (`top`, `bottom`, `left`, `right`, negative to expand outward or positive to crop inward),
 * horizontal `stretch` proportion, and UI `size` scale. Saved per vehicle and applied live without restarting.
 */
export type ScreenFit = ScreenFitValues

export type FitEdge = 'top' | 'bottom' | 'left' | 'right'

export const NO_FIT: ScreenFit = { top: 0, bottom: 0, left: 0, right: 0, stretch: 1, size: 1 }

/** How far each manual setting goes. Negative edges expand past the initial bounds; positive edges pull inward. */
export const FIT_LIMITS = {
  edge: { min: -0.25, max: 0.45, step: 0.005 },
  stretch: { min: 0.5, max: 2.0, step: 0.01 },
  size: { min: 0.5, max: 1.8, step: 0.05 },
} as const

export type ScreenFitPresetId = 'auto' | 'widescreen' | 'sixteenTen' | 'classic' | 'ultrawide' | 'bandCenter'

/** Quick one-tap presets for unmapped modded truck screens of common aspect ratios and UV bands. */
export const SCREEN_FIT_PRESETS: readonly { id: ScreenFitPresetId; fit: ScreenFit }[] = [
  { id: 'auto', fit: NO_FIT },
  { id: 'widescreen', fit: { top: 0, bottom: 0, left: 0, right: 0, stretch: 0.9, size: 1 } },
  { id: 'sixteenTen', fit: { top: 0, bottom: 0, left: 0, right: 0, stretch: 1, size: 1 } },
  { id: 'classic', fit: { top: 0, bottom: 0, left: 0, right: 0, stretch: 1.2, size: 1.1 } },
  { id: 'ultrawide', fit: { top: 0, bottom: 0, left: 0, right: 0, stretch: 0.65, size: 0.85 } },
  { id: 'bandCenter', fit: { top: 0.14, bottom: 0.22, left: 0.005, right: 0.005, stretch: 1, size: 1.15 } },
]

/** Which preset matches `fit`, if one does (`null` when custom-adjusted). */
export function activeFitPreset(fit: ScreenFit): ScreenFitPresetId | null {
  const found = SCREEN_FIT_PRESETS.find((preset) =>
    (Object.keys(NO_FIT) as (keyof ScreenFit)[]).every((k) => Math.abs(preset.fit[k] - fit[k]) < 0.0005),
  )
  return found?.id ?? null
}

/** The fit's key: the screen and the vehicle on it (`null` until a screen is drawn); one layout is in several
 * vehicles, not always shown alike. */
export function fitKey(layout: ScreenLayoutId | null, truck: TruckInfo | null): string | null {
  if (layout === null) return null
  // A mod screen's slot is setup's choice, another as the installed mods change: the vehicle's fit, whichever.
  const screen = layout.startsWith('mod-') ? 'mod' : layout
  return `${screen}|${truck ? truck.id.replace(/^vehicle\./, '') : ''}`
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
  const spanX = Math.max(0.1, 1 - fit.left - fit.right)
  const spanY = Math.max(0.1, 1 - fit.top - fit.bottom)
  return {
    left: `${percent(fit.left)}vw`,
    top: `${percent(fit.top)}vh`,
    width: `${percent(spanX / fit.stretch)}vw`,
    height: `${percent(spanY)}vh`,
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
    if (typeof read[edge] === 'number' && Number.isFinite(read[edge])) fit[edge] = clamp(read[edge], FIT_LIMITS.edge)
  }
  if (typeof read.stretch === 'number' && Number.isFinite(read.stretch)) {
    fit.stretch = clamp(read.stretch, FIT_LIMITS.stretch)
  }
  if (typeof read.size === 'number' && Number.isFinite(read.size)) fit.size = clamp(read.size, FIT_LIMITS.size)
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

/**
 * The fit on the cab's screen as the phones know it: Adjust Screen is on the phone in the game and on the player's
 * real phone too; the cab's page tells them.
 */
export const useShownFit = create<{ key: string | null; fit: ScreenFit }>()(() => ({ key: null, fit: NO_FIT }))

/** The cab's page's own fit, for the phones (`screenFitShown`). */
export function announceFit(key: string | null, fit: ScreenFit): void {
  if (isInHost()) postToHost({ type: 'screenFitShown', key, fit })
}

/** A phone's Adjust Screen (`screenFit`), in the cab's page alone: `key` the cab's screen's; told back. */
export function applyPhoneFit(key: string | null, change: Partial<ScreenFit> | undefined, reset: boolean): void {
  const fits = useScreenFits.getState()
  if (key !== null && reset) fits.reset(key)
  else if (key !== null && change) fits.change(key, change)
  announceFit(key, key === null ? NO_FIT : (useScreenFits.getState().fits[key] ?? NO_FIT))
}
