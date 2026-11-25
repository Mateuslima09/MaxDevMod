import { create } from 'zustand'
import { createJSONStorage, persist } from 'zustand/middleware'
import { IS_DEMO } from '../../shared/host'

/** The beeps' volume steps, of the loudest they were made (the 100%, as they were before it could be set). */
export const BEEP_VOLUMES = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1, 1.25, 1.5]
export const DEFAULT_BEEP_VOLUME = BEEP_VOLUMES.indexOf(1)

/** A step kept on the PC, or the default if it isn't one. */
export function beepVolumeStep(value: unknown): number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 && value < BEEP_VOLUMES.length
    ? value
    : DEFAULT_BEEP_VOLUME
}

/** The reversing camera's choices (Settings, Reversing Camera), kept on this PC: shown when reversing, the
 * sensors' beeps and how loud (owner 2026-11-14), the guide lines. All on at first, as a new truck comes. */
interface ReversingSettings {
  camera: boolean
  beeps: boolean
  guides: boolean
  /** The beeps' volume, one of BEEP_VOLUMES' steps (0: the softest). */
  beepVolume: number
  set(choice: Partial<Pick<ReversingSettings, 'camera' | 'beeps' | 'guides' | 'beepVolume'>>): void
}

const STORAGE_KEY = IS_DEMO ? 'westplay.reversing.demo' : 'westplay.reversing'

export const useReversingSettings = create<ReversingSettings>()(
  persist(
    (set) => ({
      camera: true,
      beeps: true,
      guides: true,
      beepVolume: DEFAULT_BEEP_VOLUME,
      set: (choice) => set(choice),
    }),
    {
      name: STORAGE_KEY,
      storage: createJSONStorage(() => localStorage),
      partialize: ({ camera, beeps, guides, beepVolume }) => ({ camera, beeps, guides, beepVolume }),
      merge: (stored, current) => {
        const kept = (stored ?? {}) as Partial<Record<'camera' | 'beeps' | 'guides' | 'beepVolume', unknown>>
        const flag = (value: unknown, otherwise: boolean) => (typeof value === 'boolean' ? value : otherwise)
        return {
          ...current,
          camera: flag(kept.camera, current.camera),
          beeps: flag(kept.beeps, current.beeps),
          guides: flag(kept.guides, current.guides),
          beepVolume: beepVolumeStep(kept.beepVolume),
        }
      },
    },
  ),
)

// Keep phone/iPad CameraApp and cabin ReversingCamera live-synced across WebView2 windows.
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key !== STORAGE_KEY || !event.newValue) return
    try {
      const parsed = JSON.parse(event.newValue) as { state?: Partial<ReversingSettings> }
      const s = parsed.state
      if (!s) return
      useReversingSettings.setState((cur) => ({
        camera: typeof s.camera === 'boolean' ? s.camera : cur.camera,
        beeps: typeof s.beeps === 'boolean' ? s.beeps : cur.beeps,
        guides: typeof s.guides === 'boolean' ? s.guides : cur.guides,
        beepVolume: s.beepVolume !== undefined ? beepVolumeStep(s.beepVolume) : cur.beepVolume,
      }))
    } catch {
      // Ignore malformed storage updates.
    }
  })
}
