import { useCallback, useState } from 'react'
import { useKeyName } from '../keys'
import { Group, Note, Panel, Row, Slider, Switch, Tick } from '../room/parts'
import { n, useWords } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import { type Binding, CHARACTER_KEY, CHARACTER_PEOPLE, CHARACTER_SENSITIVITY, type KeyCode } from '../useSettings'
import { KeyButton } from './KeyButton'
import type { PageProps } from './page'

/**
 * The Modo Personagem (docs/personagem): walking about on foot by the truck, through the game's own walking camera.
 * On or off (off at first: the plugin does nothing with it off), with what its key does (F8 at first, any other
 * the player chooses but the ones walking uses); the mouse's speed while walking (the game's camera turns too fast
 * for walking, owner 2026-11-04), which `-` and `=` change in the game too; who walks (WestPlay's own people,
 * chosen by their picture: owner 2026-11-05); and what it needs: single player, and the game's developer mode, which
 * WestPlay sets with the game closed.
 */
export function Character({ settings: state, cw, status }: PageProps) {
  const w = cw.character
  const words = useWords()
  const { settings, update, error } = state
  useScreenAt('home', GLOW.home, null)
  const character = settings?.character ?? {
    enabled: true,
    sensitivity: 50,
    key: CHARACTER_KEY,
    person: CHARACTER_PEOPLE.default,
    hints: true,
    hintsKey: 0,
  }
  const keyName = useKeyName(character.key, words)
  const [listening, setListening] = useState<'walk' | 'hints' | null>(null)
  const [refused, setRefused] = useState<string | null>(null)
  // The person changed while the game was open: it takes the game restarting (its mod is read as it starts), said
  // until the game closes (owner 2026-11-06).
  const gameOpen = status?.gameConnected === true
  const [changedOpen, setChangedOpen] = useState(false)
  if (changedOpen && !gameOpen) setChangedOpen(false)
  const listen = useCallback(
    (which: 'walk' | 'hints') => (on: boolean) => {
      setListening(on ? which : null)
      if (on) setRefused(null)
    },
    [],
  )
  const chooseHintsKey = (binding: Binding) => {
    if (!settings) return
    // Backspace: no key.
    const key = binding.key === BACKSPACE ? 0 : binding.key
    const taken = key === 0 ? null : key === character.key ? 'walking' : takenFor(key, settings.interaction.key)
    if (taken) {
      setRefused(cw.controls.inUse.replaceAll('{action}', taken === 'screen' ? cw.controls.useScreen : w.walking))
      return
    }
    setRefused(null)
    update({ ...settings, character: { ...character, hintsKey: key } })
  }
  const chooseKey = (binding: Binding) => {
    if (!settings) return
    const taken = binding.key === character.hintsKey ? 'walking' : takenFor(binding.key, settings.interaction.key)
    if (taken) {
      setRefused(cw.controls.inUse.replaceAll('{action}', taken === 'screen' ? cw.controls.useScreen : w.walking))
      return
    }
    setRefused(null)
    update({ ...settings, character: { ...character, key: binding.key } })
  }
  return (
    <Panel place="page" title={w.title} sub={w.sub}>
      {settings && (
        <Group title={w.title} place={4}>
          <Row
            title={w.enable}
            detail={w.enableDetail.replaceAll('{key}', keyName)}
            accessory={
              <Switch
                on={character.enabled}
                label={w.enable}
                change={(enabled) => update({ ...settings, character: { ...character, enabled } })}
              />
            }
          />
          <Row
            title={w.walkKey}
            accessory={
              <KeyButton
                binding={{ key: character.key, withInteractionKey: false }}
                label={w.walkKey}
                listening={listening === 'walk'}
                listen={listen('walk')}
                choose={chooseKey}
                words={words}
                pressKey={cw.controls.pressKey}
              />
            }
          />
          <Row
            title={w.hints}
            detail={w.hintsDetail}
            accessory={
              <Switch
                on={character.hints}
                label={w.hints}
                change={(hints) => update({ ...settings, character: { ...character, hints } })}
              />
            }
          />
          <Row
            title={w.hintsKey}
            detail={w.hintsKeyDetail}
            accessory={
              character.hintsKey === 0 && listening !== 'hints' ? (
                <button
                  type="button"
                  className="ob-button cp-keybind"
                  data-glass
                  data-small
                  aria-label={w.hintsKey}
                  onClick={() => listen('hints')(true)}
                >
                  {w.noKey}
                </button>
              ) : (
                <KeyButton
                  binding={{ key: character.hintsKey, withInteractionKey: false }}
                  label={w.hintsKey}
                  listening={listening === 'hints'}
                  listen={listen('hints')}
                  choose={chooseHintsKey}
                  words={words}
                  pressKey={cw.controls.pressKey}
                />
              )
            }
          />
          <PercentRow
            saved={character.sensitivity}
            label={w.speed}
            detail={w.speedDetail}
            save={(sensitivity) => update({ ...settings, character: { ...character, sensitivity } })}
          />
        </Group>
      )}
      {refused && <Note tone="bad">{refused}</Note>}
      <Note tone="calm">{w.note}</Note>
      {settings && (
        <>
          <p className="ob-group-title ob-in" style={n(6)}>
            {w.person}
          </p>
          <div className="ob-people ob-in" style={n(6)}>
            {CHARACTER_PEOPLE.people.map((person, place) => {
              const chosen = character.person === place
              return (
                <button
                  key={person.picture}
                  type="button"
                  className="ob-person"
                  data-chosen={chosen || undefined}
                  aria-pressed={chosen}
                  onClick={() => {
                    if (chosen) return
                    update({ ...settings, character: { ...character, person: place } })
                    if (gameOpen) setChangedOpen(true)
                  }}
                >
                  <img src={person.picture} alt="" draggable={false} />
                  <span className="ob-person-name">
                    {personName(w, place)}
                    {chosen && <Tick />}
                  </span>
                </button>
              )
            })}
          </div>
          {gameOpen ? (
            <Note tone={changedOpen ? 'bad' : 'calm'}>{w.restartNote}</Note>
          ) : (
            <Note tone="calm">{w.personNote}</Note>
          )}
        </>
      )}
      {error && <Note tone="bad">{error}</Note>}
    </Panel>
  )
}

/** A person's name under their picture: "Man 1", "Woman 1"… each kind numbered from 1, in the page's language. */
function personName(w: PageProps['cw']['character'], place: number): string {
  const people = CHARACTER_PEOPLE.people
  const kind = people[place]?.kind ?? 'man'
  const n = people.slice(0, place + 1).filter((person) => person.kind === kind).length
  return (kind === 'man' ? w.man : w.woman).replaceAll('{n}', String(n))
}

const BACKSPACE: KeyCode = 0x08

/** The keys walking uses (W, A, S, D, Shift, Space, Ctrl, V for first/third person, `-` and `=` for the mouse's
 * speed, and 0–9, the game's cameras, kept from it while walking), and the screen's key: none of them gets out. */
const WALKING_KEYS: KeyCode[] = [
  0x57,
  0x41,
  0x53,
  0x44,
  0x10,
  0xa0,
  0xa1,
  0x20,
  0x11,
  0xa2,
  0xa3,
  0x56,
  0xbd,
  0xbb,
  ...Array.from({ length: 10 }, (_, digit) => 0x30 + digit),
]

/** What `key` is already taken for, if anything: walking, or the screen's key. */
function takenFor(key: KeyCode, screenKey: KeyCode): 'walking' | 'screen' | null {
  if (WALKING_KEYS.includes(key)) return 'walking'
  if (key === screenKey) return 'screen'
  return null
}

/** The mouse's speed while walking, in percent: its slider follows the thumb, saved once it's let go (one saved
 * elsewhere, or changed in the game, resets it). */
function PercentRow({
  saved,
  label,
  detail,
  save,
}: {
  saved: number
  label: string
  detail: string
  save(value: number): void
}) {
  const [dragging, setDragging] = useState(saved)
  const [lastSaved, setLastSaved] = useState(saved)
  if (saved !== lastSaved) {
    setLastSaved(saved)
    setDragging(saved)
  }
  return (
    <div className="ob-row cp-slider-row">
      <span className="ob-row-text">
        <b>{label}</b>
        <span>{detail}</span>
      </span>
      <span className="ob-row-value">{`${Math.round(dragging)}%`}</span>
      <Slider
        value={dragging}
        min={CHARACTER_SENSITIVITY.min}
        max={CHARACTER_SENSITIVITY.max}
        step={CHARACTER_SENSITIVITY.step}
        label={label}
        change={setDragging}
        commit={save}
      />
    </div>
  )
}
