import { Hold } from './CommandsApp'
import { List, Row } from './ios'

/**
 * Services (owner 2026-11-03: "call the game's services, a tow truck, to the nearest service"): the game's own
 * services window (its F7, `services`), opened from the phone, and parking by itself (`quickpark`), as the truck's
 * commands are given (crates/plugin commands.rs). The window's choices the game takes only from its own screen:
 * the player picks there.
 */
export function ServicesApp({ words }: { words: ServicesWords }) {
  return (
    <>
      <section className="handset-commands tone-orange handset-services">
        <div className="handset-command-grid">
          <Hold command="services" label={words.call} glyph={TOW} wide />
          <Hold command="quickpark" label={words.park} glyph={PARK} wide />
        </div>
      </section>
      <List header={words.how}>
        <Row label={<span className="ios-row-wrap">{words.howText}</span>} />
      </List>
    </>
  )
}

export interface ServicesWords {
  call: string
  park: string
  how: string
  howText: string
}

const TOW = (
  <path d="M2 15V9h7l2 3h3V7l7 5v3h-2a2.5 2.5 0 0 0-5 0H9a2.5 2.5 0 0 0-5 0Zm4.5-1a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Zm10 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Z" />
)
const PARK = <path d="M5 3h8a6 6 0 0 1 0 12H9v6H5Zm4 4v4h4a2 2 0 0 0 0-4Z" fillRule="evenodd" />
