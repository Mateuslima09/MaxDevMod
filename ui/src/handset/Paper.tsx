import { type PointerEvent, type ReactNode, useEffect, useRef, useState } from 'react'
import { useLive } from '../cabin/state/store'
import { formatGameTime } from '../shared/format'
import { cabClock, postToHost } from '../shared/host'
import { isSigned, PAPER_WORDS, paperNumber, tonnes } from './paperWords'
import { distance, formatMoney, type Prefs } from './prefs'

/** Where the driver's signature is kept from the pick-up, for the delivery's paper (this device's own). */
const SIGNATURE_KEY = 'westplay-handset-signature'
/** How long the recipient takes to sign, and the stamp to come down after a signature (ms). */
const RECIPIENT_SIGNS_FOR = 1700
const STAMPED_AFTER = 450

let audio: AudioContext | null = null

/** The stamp coming down on the paper: a short, dull knock. */
function thump(): void {
  try {
    audio ??= new AudioContext()
    const context = audio
    if (context.state === 'suspended') void context.resume()
    const at = context.currentTime + 0.01
    const oscillator = context.createOscillator()
    const gain = context.createGain()
    oscillator.type = 'triangle'
    oscillator.frequency.setValueAtTime(150, at)
    oscillator.frequency.exponentialRampToValueAtTime(48, at + 0.14)
    gain.gain.setValueAtTime(0.0001, at)
    gain.gain.exponentialRampToValueAtTime(0.5, at + 0.008)
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.22)
    oscillator.connect(gain).connect(context.destination)
    oscillator.start(at)
    oscillator.stop(at + 0.25)
  } catch {
    // No sound here: the stamp still shows.
  }
}

/**
 * The cargo's paper in the phone's frame, in place of the phone (owner 2026-11-11): the player walked up to the
 * clerk by the trailer and asked for it (the plugin's `freight`), and the host shows this large in the middle of the
 * screen, with the mouse. At the pick-up, the nota fiscal with the cargo's data: the driver signs it with the mouse,
 * the clerk stamps it, and it's collected; only then can the trailer be coupled. At the delivery, the same paper: the
 * recipient signs and stamps it, and the receipt is collected; only then can the cargo be delivered. Put down
 * (closed) without collecting, nothing is released.
 */
export function Paper({
  stage,
  language,
  prefs,
}: {
  stage: 'pickup' | 'delivery'
  language: 'en' | 'pt'
  prefs: Prefs
}) {
  const words = PAPER_WORDS[language]
  const job = useLive((state) => state.job)
  const truck = useLive((state) => state.truck)
  const gameCurrency = useLive((state) => state.wallet?.currency ?? 'USD')
  const time = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : null))
  // Signing, then stamped (after a moment), then collected.
  const [signed, setSigned] = useState(false)
  const [stamped, setStamped] = useState(false)
  const [signing, setSigning] = useState(false)
  useEffect(() => {
    if (!signed) return
    const timer = setTimeout(() => {
      setStamped(true)
      thump()
    }, STAMPED_AFTER)
    return () => clearTimeout(timer)
  }, [signed])

  const done = (collected: boolean) => postToHost({ type: 'paperDone', collected })
  const issued = time === null ? '—' : formatGameTime(time)
  const pickup = stage === 'pickup'

  return (
    <article className="paper">
      <button type="button" className="paper-close" aria-label={words.close} onClick={() => done(false)}>
        ×
      </button>
      <header className="paper-head">
        <div>
          <h1>{words.title}</h1>
          <p>{words.subtitle}</p>
        </div>
        <dl>
          <dt>{words.number}</dt>
          <dd>{job ? paperNumber(job) : '000000'}</dd>
          <dt>{words.issued}</dt>
          <dd>{issued}</dd>
        </dl>
      </header>

      <section className="paper-grid">
        <Field label={words.shipper} wide>
          <b>{job?.special ? job.sourceCity : job?.sourceCompany || '—'}</b>
          <span>{job?.sourceCity}</span>
        </Field>
        <Field label={words.recipient} wide>
          <b>{job?.special ? job.destinationCity : job?.destinationCompany || '—'}</b>
          <span>{job?.destinationCity}</span>
        </Field>
        <Field label={words.cargo} wide>
          <b>{job?.cargo || '—'}</b>
        </Field>
        <Field label={words.weight}>
          <b>{job ? tonnes(job.cargoMass, language) : '—'}</b>
        </Field>
        <Field label={words.distance}>
          <b>{job ? distance(job.plannedDistanceKm * 1000, prefs.units) : '—'}</b>
        </Field>
        <Field label={words.freight} wide>
          <b>{job ? formatMoney(job.income, gameCurrency, prefs.currency, language) : '—'}</b>
        </Field>
        <Field label={words.vehicle} wide>
          <b>{[truck?.brand, truck?.name].filter(Boolean).join(' ') || '—'}</b>
        </Field>
      </section>

      {pickup ? (
        <section className="paper-sign">
          <SignaturePad
            hint={words.signHere}
            clear={words.clear}
            sign={words.sign}
            locked={signed}
            onSigned={(picture) => {
              try {
                localStorage.setItem(SIGNATURE_KEY, picture)
              } catch {
                // Not kept: the delivery's paper shows no driver's signature.
              }
              setSigned(true)
            }}
          />
          <p className="paper-line">{words.driverSigns}</p>
          {stamped && (
            <Stamp top={words.stampChecked} bottom={words.stampReleased} who={job?.sourceCompany} when={issued} />
          )}
        </section>
      ) : (
        <section className="paper-sign">
          <p className="paper-declares">{words.received}</p>
          <div className="paper-signatures">
            <div>
              <div className="paper-kept">
                <KeptSignature />
              </div>
              <p className="paper-line">{words.driver}</p>
            </div>
            <div>
              <div className="paper-kept">{(signing || signed) && <RecipientSignature />}</div>
              <p className="paper-line">{words.recipientSigns}</p>
            </div>
          </div>
          {stamped && <Stamp top={words.stampReceived} who={job?.destinationCompany} when={issued} />}
        </section>
      )}

      <footer className="paper-foot">
        {stamped ? (
          <>
            <p>{pickup ? words.collectHint : words.collectReceiptHint}</p>
            <button type="button" className="paper-button main" onClick={() => done(true)}>
              {pickup ? words.collect : words.collectReceipt}
            </button>
          </>
        ) : (
          !pickup && (
            <button
              type="button"
              className="paper-button main"
              disabled={signing}
              onClick={() => {
                setSigning(true)
                setTimeout(() => setSigned(true), RECIPIENT_SIGNS_FOR)
              }}
            >
              {words.handOver}
            </button>
          )
        )}
      </footer>
    </article>
  )
}

function Field({ label, wide = false, children }: { label: string; wide?: boolean; children: ReactNode }) {
  return (
    <div className={`paper-field${wide ? ' wide' : ''}`}>
      <small>{label}</small>
      {children}
    </div>
  )
}

/** The clerk's stamp, come down askew on the paper. */
function Stamp({ top, bottom, who, when }: { top: string; bottom?: string; who?: string; when: string }) {
  return (
    <div className="paper-stamp" aria-hidden>
      <b>{top}</b>
      {bottom && <b>{bottom}</b>}
      <span>{[who, when].filter(Boolean).join(' · ')}</span>
    </div>
  )
}

/**
 * The box the driver signs in, with the mouse held down (the pointer the host gives the page): its line drawn as
 * the mouse goes, enough of it making a signature (`isSigned`). Signed, it's kept as it is, as a picture.
 */
function SignaturePad({
  hint,
  clear,
  sign,
  locked,
  onSigned,
}: {
  hint: string
  clear: string
  sign: string
  locked: boolean
  onSigned: (picture: string) => void
}) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const last = useRef<{ x: number; y: number } | null>(null)
  const [ink, setInk] = useState(0)

  // Drawn at the screen's own sharpness: the paper shows larger in the game than the page is laid out.
  useEffect(() => {
    const element = canvas.current
    if (!element) return
    const scale = 3
    element.width = element.clientWidth * scale
    element.height = element.clientHeight * scale
    const context = element.getContext('2d')
    if (!context) return
    context.scale(scale, scale)
    context.lineCap = 'round'
    context.lineJoin = 'round'
    context.lineWidth = 1.7
    context.strokeStyle = '#14227a'
  }, [])

  const at = (event: PointerEvent<HTMLCanvasElement>) => {
    const box = event.currentTarget.getBoundingClientRect()
    return { x: event.clientX - box.left, y: event.clientY - box.top }
  }
  const down = (event: PointerEvent<HTMLCanvasElement>) => {
    if (locked) return
    event.currentTarget.setPointerCapture?.(event.pointerId)
    last.current = at(event)
  }
  const move = (event: PointerEvent<HTMLCanvasElement>) => {
    const from = last.current
    const context = canvas.current?.getContext('2d')
    if (locked || !from || !context) return
    const to = at(event)
    context.beginPath()
    context.moveTo(from.x, from.y)
    context.lineTo(to.x, to.y)
    context.stroke()
    last.current = to
    setInk((drawn) => drawn + Math.hypot(to.x - from.x, to.y - from.y))
  }
  const up = () => {
    last.current = null
  }
  const wipe = () => {
    const element = canvas.current
    element?.getContext('2d')?.clearRect(0, 0, element.width, element.height)
    setInk(0)
  }

  return (
    <div className="paper-pad">
      <canvas
        ref={canvas}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerLeave={up}
        onPointerCancel={up}
      />
      {ink === 0 && <span className="paper-pad-hint">{hint}</span>}
      {!locked && (
        <div className="paper-pad-buttons">
          <button type="button" className="paper-button" disabled={ink === 0} onClick={wipe}>
            {clear}
          </button>
          <button
            type="button"
            className="paper-button main"
            disabled={!isSigned(ink)}
            onClick={() => onSigned(canvas.current?.toDataURL('image/png') ?? '')}
          >
            {sign}
          </button>
        </div>
      )}
    </div>
  )
}

/** The driver's signature as they made it at the pick-up, if this device kept it. */
function KeptSignature() {
  const [picture] = useState(() => {
    try {
      return localStorage.getItem(SIGNATURE_KEY)
    } catch {
      return null
    }
  })
  return picture?.startsWith('data:image/png') ? <img src={picture} alt="" draggable={false} /> : null
}

/** The recipient signing: their hand's line, drawn as they go. */
function RecipientSignature() {
  return (
    <svg className="paper-recipient" viewBox="0 0 150 56" aria-hidden>
      <path
        pathLength={1}
        d="M8 40c6-14 12-30 17-29s-4 30 1 31 9-21 14-20 0 18 5 18 8-14 12-13 1 11 5 11 7-10 11-9 2 8 6 8c9 0 14-22 22-24s3 26 9 25 9-9 14-12 12-3 18-1M30 47c28-6 62-8 104-6"
      />
    </svg>
  )
}
