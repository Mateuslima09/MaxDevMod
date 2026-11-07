import { useCharacter } from '../cabin/state/store'
import { postToHost } from '../shared/host'

/**
 * Who walks in the Modo Personagem, chosen in the game (owner 2026-11-07: "WestPlay in front, to change the person,
 * without leaving the game"): the phone taken out on it by its keybind (the interaction key with 5 at first), each
 * person's picture to tap. The choice is the app's settings' (the companion's Personagem page shows it too), and it's
 * who gets out the next time.
 */
export function CharacterApp({ words }: { words: CharacterWords }) {
  const chosen = useCharacter((state) => state.person)
  return (
    <>
      <div className="handset-people">
        {PEOPLE.map((person, place) => {
          const here = chosen === place
          return (
            <button
              key={person.picture}
              type="button"
              className="handset-person"
              data-chosen={here || undefined}
              aria-pressed={here}
              onClick={() => {
                if (here) return
                // Shown chosen at once; the settings coming back say so too.
                useCharacter.setState({ person: place })
                postToHost({ type: 'characterPerson', person: place })
              }}
            >
              <img src={person.picture} alt="" draggable={false} />
              <span>{name(words, place)}</span>
            </button>
          )
        })}
      </div>
      <p className="handset-people-note">{words.note}</p>
    </>
  )
}

export interface CharacterWords {
  man: string
  woman: string
  note: string
}

export const CHARACTER_WORDS: Record<'en' | 'pt', CharacterWords> = {
  en: {
    man: 'Man {n}',
    woman: 'Woman {n}',
    note: 'The one chosen gets out the next time you leave the truck.',
  },
  pt: {
    man: 'Homem {n}',
    woman: 'Mulher {n}',
    note: 'O escolhido desce na próxima vez que você sair do caminhão.',
  },
}

/** WestPlay's own people, in the bridge's `CHARACTER_PEOPLE` order (as the companion's `CHARACTER_PEOPLE`). */
const PEOPLE = [
  { picture: '/characters/wt.png', kind: 'man' },
  { picture: '/characters/wm.png', kind: 'woman' },
  { picture: '/characters/wc.png', kind: 'man' },
] as const

/** "Man 1", "Woman 1"… each kind numbered from 1. */
function name(words: CharacterWords, place: number): string {
  const kind = PEOPLE[place]?.kind ?? 'man'
  const n = PEOPLE.slice(0, place + 1).filter((person) => person.kind === kind).length
  return (kind === 'man' ? words.man : words.woman).replaceAll('{n}', String(n))
}
