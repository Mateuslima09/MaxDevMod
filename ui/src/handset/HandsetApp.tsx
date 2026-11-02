import { useEffect } from 'react'
import { formatGameTime } from '../shared/format'
import { cabClock } from '../shared/host'
import { MapView } from '../cabin/dashboard/MapView'
import { NowPlayingWidget } from '../cabin/dashboard/widgets'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import '../cabin/dashboard/dashboard.css'

/**
 * The phone in the game (owner 2026-11-01: "a phone that shows up in the game at a key"), drawn by the plugin over
 * the game's picture while it's out (plugin handset.rs): the time in the game, the map following the truck, what
 * plays, and the job: its cargo, its weight, where it goes and how far. From the same host messages the cab has;
 * nothing of the cab's own doing runs here (the cab does it, once).
 */
export function HandsetApp() {
  useEffect(() => connectHost(), [])
  useEffect(() => followTimeZone(), [])
  const language = languageOf(useLive((state) => state.gameLanguage))
  useEffect(() => {
    document.documentElement.lang = language
  }, [language])
  useLookChanges()
  const { appearance } = useLook()
  const words = WORDS[language === 'pt' ? 'pt' : 'en']
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))

  return (
    <main className={`handset shell appearance-${appearance}`}>
      <div className="handset-screen">
        <header className="handset-status">
          <span className="handset-time">{time === null ? '--:--' : formatGameTime(time)}</span>
          <span className="handset-island" />
          <span className="handset-signal" aria-hidden>
            <svg viewBox="0 0 34 12" width="34" height="12">
              <rect x="0" y="8" width="3" height="4" rx="1" />
              <rect x="5" y="5" width="3" height="7" rx="1" />
              <rect x="10" y="2" width="3" height="10" rx="1" />
              <rect x="20" y="1" width="12" height="10" rx="3" fill="none" stroke="currentColor" strokeWidth="1.4" />
              <rect x="22" y="3" width="8" height="6" rx="1.5" />
            </svg>
          </span>
        </header>
        <div className="handset-body">
          <div className="handset-map">
            <MapView />
          </div>
          <Freight words={words} />
          <div className="handset-music">
            <NowPlayingWidget />
          </div>
        </div>
        <div className="handset-home" />
      </div>
    </main>
  )
}

/** The job, as the game gives it: the cargo and its weight, from where to where, how far yet, what it pays. */
function Freight({ words }: { words: Words }) {
  const job = useLive((state) => state.job)
  const left = useLive((state) => state.telemetry?.navigationDistance ?? 0)
  if (!job) {
    return (
      <article className="handset-card handset-freight">
        <div className="handset-kicker">{words.freight}</div>
        <div className="handset-none">{words.noJob}</div>
      </article>
    )
  }
  const from = job.special ? job.sourceCity : `${job.sourceCompany} · ${job.sourceCity}`
  const to = job.special ? job.destinationCity : `${job.destinationCompany} · ${job.destinationCity}`
  return (
    <article className="handset-card handset-freight">
      <div className="handset-kicker">{job.cargoLoaded ? words.delivering : words.pickingUp}</div>
      <div className="handset-cargo">{job.cargo}</div>
      <div className="handset-facts">
        <span>{(job.cargoMass / 1000).toLocaleString(undefined, { maximumFractionDigits: 1 })} t</span>
        {left > 0 && <span>{Math.round(left / 1000).toLocaleString()} km</span>}
        {job.income > 0 && <span>$ {job.income.toLocaleString()}</span>}
      </div>
      <div className="handset-route">
        <span>{from}</span>
        <span className="handset-arrow">↓</span>
        <span>{to}</span>
      </div>
    </article>
  )
}

const WORDS = {
  en: {
    freight: 'Job',
    noJob: 'No job right now',
    delivering: 'Delivering',
    pickingUp: 'Picking up',
  },
  pt: {
    freight: 'Frete',
    noJob: 'Nenhum frete agora',
    delivering: 'Entregando',
    pickingUp: 'Indo buscar',
  },
} satisfies Record<'en' | 'pt', Record<string, string>>

type Words = (typeof WORDS)['en']
