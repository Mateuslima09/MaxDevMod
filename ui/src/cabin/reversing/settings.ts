import { create } from 'zustand'
import { createJSONStorage, persist } from 'zustand/middleware'
import { IS_DEMO } from '../../shared/host'

/** The reversing camera's choices (Settings, Reversing Camera), kept on this PC: shown when reversing, the
 * sensors' beeps, the guide lines. All on at first, as a new truck comes. */
interface ReversingSettings {
  camera: boolean
  beeps: boolean
  guides: boolean
  set(choice: Partial<Pick<ReversingSettings, 'camera' | 'beeps' | 'guides'>>): void
}

export const useReversingSettings = create<ReversingSettings>()(
  persist(
    (set) => ({
      camera: true,
      beeps: true,
      guides: true,
      set: (choice) => set(choice),
    }),
    {
      name: IS_DEMO ? 'westplay.reversing.demo' : 'westplay.reversing',
      storage: createJSONStorage(() => localStorage),
      partialize: ({ camera, beeps, guides }) => ({ camera, beeps, guides }),
      merge: (stored, current) => {
        const kept = (stored ?? {}) as Partial<Record<'camera' | 'beeps' | 'guides', unknown>>
        const flag = (value: unknown, otherwise: boolean) => (typeof value === 'boolean' ? value : otherwise)
        return {
          ...current,
          camera: flag(kept.camera, current.camera),
          beeps: flag(kept.beeps, current.beeps),
          guides: flag(kept.guides, current.guides),
        }
      },
    },
  ),
)
