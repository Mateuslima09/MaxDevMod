import { useState } from 'react'
import { AppScreen } from '../cabin/apps/AppScreen'
import { useAppIcon } from '../cabin/state/hooks'
import { type AppId, APPS, RELEASED_APP_IDS } from '../cabin/state/apps'
import type { Route } from '../cabin/state/navigation'
import { useLanguage, useText } from '../cabin/state/text'

/**
 * The phone panel's split screen (owner 2026-11-03): where the cab has the game's map beside WestPlay, the phone has
 * nothing under the page, so a second app goes there, picked from the apps the screen beside it isn't showing. The
 * sidebar's split button ends it, as in the cab. Its screens are its own, as the panel's are.
 */
export function PhoneSplitPane({ main }: { main: Route }) {
  const [picked, setPicked] = useState<AppId | null>(null)
  const mainApp = main.kind === 'app' ? main.app : null
  // The screen beside it opened the same app: the choice again (an app is drawn once).
  const shown = picked !== mainApp ? picked : null
  const words = WORDS[useLanguage() as keyof typeof WORDS] ?? WORDS.en
  return (
    <div className="phone-split-pane">
      <div className="phone-split-inner">
        {shown ? (
          <>
            <AppScreen key={shown} app={shown} />
            <button
              type="button"
              className="phone-split-change glass"
              aria-label={words.change}
              onClick={() => setPicked(null)}
            >
              <svg viewBox="0 0 24 24" width="30" height="30" aria-hidden>
                {[4, 10.25, 16.5].flatMap((x) =>
                  [4, 10.25, 16.5].map((y) => (
                    <rect key={`${x}-${y}`} x={x} y={y} width="3.5" height="3.5" rx="1" fill="currentColor" />
                  )),
                )}
              </svg>
            </button>
          </>
        ) : (
          <section className="phone-split-picker glass" aria-label={words.pick}>
            <h2>{words.pick}</h2>
            <div className="phone-split-apps">
              {RELEASED_APP_IDS.filter((app) => app !== mainApp).map((app) => (
                <PickedApp key={app} app={app} onPick={setPicked} />
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  )
}

function PickedApp({ app, onPick }: { app: AppId; onPick: (app: AppId) => void }) {
  const icon = useAppIcon(APPS[app].icon)
  const text = useText()
  return (
    <button type="button" onClick={() => onPick(app)}>
      <img src={icon} alt="" />
      <span>{text.apps[app]}</span>
    </button>
  )
}

const WORDS = {
  en: { pick: 'Choose an app', change: 'Choose another app' },
  pt: { pick: 'Escolha um app', change: 'Escolher outro app' },
  tr: { pick: 'Bir uygulama seçin', change: 'Başka bir uygulama seçin' },
}
