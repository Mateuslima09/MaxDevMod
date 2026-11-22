import { type ReactNode, useState } from 'react'
import { postToHost, type WalletEntry } from '../shared/host'
import { useLive } from '../cabin/state/store'
import { Segmented } from './ios'
import { OFFENCES } from './offences'
import { formatMoney, type Prefs, shownCurrency } from './prefs'

const LOAN_OFFERS = [
  { amount: 10_000, rate: 0.18, days: 35 },
  { amount: 50_000, rate: 0.15, days: 35 },
  { amount: 100_000, rate: 0.12, days: 70 },
  { amount: 400_000, rate: 0.12, days: 70 },
] as const

/**
 * Nupay (owner 2026-11-03 / 2026-11-22): balance, statement, and in-game bank loans (`bank_loan` units from
 * `game.sii`) with optional queued new loan applied when the game restarts.
 */
export function NupayApp({ language, currency }: { language: 'en' | 'pt'; currency: Prefs['currency'] }) {
  const wallet = useLive((state) => state.wallet)
  const [hidden, setHidden] = useState(false)
  const [tab, setTab] = useState<'history' | 'loans'>('history')
  // Clearing the history asked, waiting for its yes.
  const [confirming, setConfirming] = useState(false)
  const words = WORDS[language]
  // The game's money, in the currency chosen (its own, `auto`).
  const from = wallet?.currency ?? 'USD'
  const shown = shownCurrency(currency, from)
  const money = (amount: number, signed = false) => formatMoney(amount, from, currency, language, signed)
  const entries = wallet?.entries ?? []
  const loans = wallet?.loans ?? []
  const pendingLoan = wallet?.pendingLoan ?? null
  const income = entries.filter((entry) => entry.amount > 0).reduce((sum, entry) => sum + entry.amount, 0)
  const spent = entries.filter((entry) => entry.amount < 0).reduce((sum, entry) => sum + entry.amount, 0)
  const totalDebt = loans.reduce((sum, loan) => sum + loan.amount, 0)
  const totalDaily = loans.reduce((sum, loan) => sum + loan.dailyPayment, 0)

  return (
    <div className="nupay">
      <header className="nupay-head">
        <div className="nupay-hello">{words.account}</div>
        <button type="button" className="nupay-balance" onClick={() => setHidden(!hidden)}>
          <span className="nupay-balance-value">
            {wallet?.balance == null ? '—' : hidden ? '••••••' : money(wallet.balance)}
          </span>
          <svg viewBox="0 0 24 24" aria-label={hidden ? words.show : words.hide}>
            {hidden ? EYE_CLOSED : EYE}
          </svg>
        </button>
        {wallet?.balance == null && <p className="nupay-note">{words.noSave}</p>}
      </header>
      <div style={{ margin: '0 0 12px' }}>
        <Segmented
          options={[
            ['history', language === 'pt' ? 'Extrato' : 'Activity'],
            ['loans', language === 'pt' ? 'Financiamentos' : 'Loans'],
          ]}
          chosen={tab}
          choose={(next) => setTab(next as 'history' | 'loans')}
        />
      </div>
      {tab === 'loans' ? (
        <>
          <div className="nupay-summary">
            <article className="nupay-total out">
              <span>{language === 'pt' ? 'Saldo devedor' : 'Total owed'}</span>
              <strong>{hidden ? '••••' : money(totalDebt)}</strong>
            </article>
            <article className="nupay-total">
              <span>{language === 'pt' ? 'Parcela diária' : 'Daily payment'}</span>
              <strong>{hidden ? '••••' : money(totalDaily)}</strong>
            </article>
          </div>
          {pendingLoan && (
            <div className="ios-list" style={{ marginBottom: 12 }}>
              <div className="ios-row">
                <span className="ios-row-text">
                  <span className="ios-row-label">
                    {language === 'pt' ? 'Empréstimo solicitado' : 'Loan queued'}: {money(pendingLoan.amount)}
                  </span>
                  <span className="ios-row-detail">
                    {language === 'pt'
                      ? `${Math.round(pendingLoan.rate * 100)}% · ${pendingLoan.days} dias — feche e reabra o jogo para aplicar.`
                      : `${Math.round(pendingLoan.rate * 100)}% · ${pendingLoan.days} days — close and restart the game to apply.`}
                  </span>
                </span>
                <button
                  type="button"
                  className="nupay-clear"
                  onClick={() => postToHost({ type: 'walletLoanCancel' })}
                >
                  {words.cancel}
                </button>
              </div>
            </div>
          )}
          <div className="nupay-history-head">
            <h2 className="nupay-title">{language === 'pt' ? 'Empréstimos ativos' : 'Active loans'}</h2>
          </div>
          {loans.length === 0 ? (
            <p className="nupay-empty">
              {language === 'pt' ? 'Nenhum financiamento ativo no save atual.' : 'No active bank loans in your save.'}
            </p>
          ) : (
            <div className="ios-list" style={{ marginBottom: 14 }}>
              {loans.map((loan, index) => {
                const paidPct = Math.max(
                  0,
                  Math.min(100, Math.round(((loan.originalAmount - loan.amount) / Math.max(1, loan.originalAmount)) * 100)),
                )
                return (
                  <div key={index} className="ios-row" style={{ flexDirection: 'column', alignItems: 'stretch', gap: 6 }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                      <span className="ios-row-label">{hidden ? '••••' : money(loan.amount)}</span>
                      <span className="ios-row-detail">
                        {language === 'pt'
                          ? `${money(loan.dailyPayment)}/dia · ${Math.round(loan.interestRate * 100)}%`
                          : `${money(loan.dailyPayment)}/day · ${Math.round(loan.interestRate * 100)}%`}
                      </span>
                    </div>
                    <div style={{ height: 5, borderRadius: 999, background: 'var(--ios-fill)', overflow: 'hidden' }}>
                      <div style={{ width: `${paidPct}%`, height: '100%', background: 'var(--ios-tint)' }} />
                    </div>
                    <span className="ios-row-detail">
                      {language === 'pt'
                        ? `Original: ${money(loan.originalAmount)} · ${paidPct}% pago (${loan.duration} dias)`
                        : `Original: ${money(loan.originalAmount)} · ${paidPct}% paid (${loan.duration} days)`}
                    </span>
                  </div>
                )
              })}
            </div>
          )}
          <div className="nupay-history-head">
            <h2 className="nupay-title">{language === 'pt' ? 'Novo empréstimo' : 'Request loan'}</h2>
          </div>
          <div className="ios-list">
            {LOAN_OFFERS.map((offer) => {
              const daily = Math.round((offer.amount * (1 + offer.rate)) / offer.days)
              return (
                <button
                  key={offer.amount}
                  type="button"
                  className="ios-row ios-row-button"
                  onClick={() => postToHost({ type: 'walletLoan', amount: offer.amount, days: offer.days, rate: offer.rate })}
                >
                  <span className="ios-row-text">
                    <span className="ios-row-label">{money(offer.amount)}</span>
                    <span className="ios-row-detail">
                      {language === 'pt'
                        ? `${Math.round(offer.rate * 100)}% juros · ${offer.days} dias · ${money(daily)}/dia`
                        : `${Math.round(offer.rate * 100)}% APR · ${offer.days} days · ${money(daily)}/day`}
                    </span>
                  </span>
                  <span style={{ color: 'var(--ios-tint)', fontWeight: 600, fontSize: 13 }}>
                    {language === 'pt' ? 'Solicitar' : 'Request'}
                  </span>
                </button>
              )
            })}
          </div>
          <p className="nupay-foot">
            {language === 'pt'
              ? 'Solicitações são gravadas no seu save (com backup .bak) assim que o jogo fechar.'
              : 'Loan requests are written to your save (with a .bak backup) as soon as the game closes.'}
          </p>
        </>
      ) : (
        <>
          <div className="nupay-summary">
            <article className="nupay-total in">
              <span>{words.income}</span>
              <strong>{hidden ? '••••' : money(income)}</strong>
            </article>
            <article className="nupay-total out">
              <span>{words.spent}</span>
              <strong>{hidden ? '••••' : money(-spent)}</strong>
            </article>
          </div>
          <div className="nupay-history-head">
            <h2 className="nupay-title">{words.history}</h2>
            {entries.length > 0 && !confirming && (
              <button type="button" className="nupay-clear" onClick={() => setConfirming(true)}>
                {words.clear}
              </button>
            )}
          </div>
          {/* Asked once more before it goes: it can't come back (owner 2026-11-03). */}
          {confirming && entries.length > 0 && (
            <div className="nupay-confirm" role="alertdialog" aria-label={words.clearAsk}>
              <p>
                <strong>{words.clearAsk}</strong>
                <span>{words.clearNote}</span>
              </p>
              <div className="nupay-confirm-buttons">
                <button type="button" onClick={() => setConfirming(false)}>
                  {words.cancel}
                </button>
                <button
                  type="button"
                  className="danger"
                  onClick={() => {
                    postToHost({ type: 'walletClear' })
                    setConfirming(false)
                  }}
                >
                  {words.clearYes}
                </button>
              </div>
            </div>
          )}
          {entries.length === 0 ? (
            <p className="nupay-empty">{words.empty}</p>
          ) : (
            byDay(entries, language).map(([day, list]) => (
              <section key={day} className="nupay-day">
                <h3>{day}</h3>
                <div className="ios-list">
                  {list.map((entry, index) => (
                    <div key={`${entry.at}-${index}`} className="ios-row nupay-row">
                      <span className={`nupay-kind nupay-kind-${entry.kind}`} aria-hidden>
                        <svg viewBox="0 0 24 24">{KIND_GLYPHS[entry.kind]}</svg>
                      </span>
                      <span className="ios-row-text">
                        <span className="ios-row-label">{kindName(entry, words)}</span>
                        <span className="ios-row-detail">{detailOf(entry, words)}</span>
                      </span>
                      <span className={`nupay-amount${entry.amount > 0 ? ' in' : ''}`}>
                        {hidden ? '••••' : money(entry.amount, true)}
                      </span>
                    </div>
                  ))}
                </div>
              </section>
            ))
          )}
          <p className="nupay-foot">
            {words.foot}
            {shown !== from && ` ${words.converted}`}
          </p>
        </>
      )}
    </div>
  )
}

type Words = (typeof WORDS)['en']

/** The entries by the day they came (the PC's), the newest first: "Today", "Yesterday", or the date. */
function byDay(entries: WalletEntry[], language: 'en' | 'pt'): [string, WalletEntry[]][] {
  const words = WORDS[language]
  const days = new Map<string, WalletEntry[]>()
  const today = new Date().toDateString()
  const yesterday = new Date(Date.now() - 86_400_000).toDateString()
  for (const entry of entries) {
    const date = new Date(entry.at)
    const key = date.toDateString()
    const name =
      key === today
        ? words.today
        : key === yesterday
          ? words.yesterday
          : date.toLocaleDateString(language === 'pt' ? 'pt-BR' : 'en-US', { day: 'numeric', month: 'long' })
    days.set(name, [...(days.get(name) ?? []), entry])
  }
  return [...days]
}

function kindName(entry: WalletEntry, words: Words): string {
  if (entry.kind === 'other') return entry.amount < 0 ? words.otherOut : words.otherIn
  return words.kinds[entry.kind]
}

/** What an entry was about: the cargo and where to, the offence, the crossing; for `other`, what it may be. */
function detailOf(entry: WalletEntry, words: Words): string {
  if (entry.kind === 'fined') return words.offences[entry.detail as keyof Words['offences']] ?? entry.detail
  if (entry.kind === 'other') return entry.amount < 0 ? words.otherOutDetail : words.otherInDetail
  if (entry.kind === 'toll') return words.tollDetail
  return entry.detail
}

const EYE = <path d="M12 5C6 5 2 12 2 12s4 7 10 7 10-7 10-7-4-7-10-7Zm0 3.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7Z" />
const EYE_CLOSED = (
  <path d="M3 4.5 4.5 3l16.5 16.5-1.5 1.5-3.2-3.2A10.6 10.6 0 0 1 12 19C6 19 2 12 2 12a17 17 0 0 1 4.2-4.6ZM12 5c6 0 10 7 10 7a17 17 0 0 1-2.9 3.7L8.6 5.6A10 10 0 0 1 12 5Z" />
)

const KIND_GLYPHS: Record<WalletEntry['kind'], ReactNode> = {
  delivered: (
    <path
      d="M3 7.5 12 3.5l9 4v9l-9 4-9-4Zm9 4v9M3 7.5l9 4 9-4"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinejoin="round"
    />
  ),
  cancelled: (
    <path
      d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM8 8l8 8M16 8l-8 8"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
    />
  ),
  fined: <path d="M12 2 1 21h22Zm-1 7h2v6h-2Zm0 8h2v2h-2Z" />,
  toll: <path d="M4 20V8l8-5 8 5v12h-4v-6H8v6Z" />,
  ferry: (
    <path
      d="M3 15h18l-2 4H5Zm3-2V8h12v5M10 8V5h4v3"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinejoin="round"
    />
  ),
  train: (
    <path
      d="M7 3h10a3 3 0 0 1 3 3v9a3 3 0 0 1-3 3l2 3h-2.5l-2-3h-5l-2 3H5l2-3a3 3 0 0 1-3-3V6a3 3 0 0 1 3-3Zm-1 4v4h12V7Zm2 7a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Zm8 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Z"
      fillRule="evenodd"
    />
  ),
  fuel: (
    <path d="M5 3h8a1 1 0 0 1 1 1v7h1a2 2 0 0 1 2 2v4a1 1 0 0 0 2 0V9l-2-2 1.4-1.4L21 8.2V17a3 3 0 0 1-6 0v-4h-1v8H4V4a1 1 0 0 1 1-1Zm1 2v5h6V5Z" />
  ),
  other: (
    <path d="M4 6h16v12H4Zm0 4h16M7 14h4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
  ),
}

const WORDS = {
  en: {
    account: 'Account',
    show: 'Show balance',
    hide: 'Hide balance',
    noSave: 'The balance shows once the game saves (an autosave too).',
    income: 'Money in',
    spent: 'Money out',
    history: 'History',
    clear: 'Clear',
    clearAsk: 'Clear the whole history?',
    clearNote: 'The balance stays as it is. This can’t be undone.',
    clearYes: 'Clear history',
    cancel: 'Cancel',
    empty: 'Nothing yet: deliveries, fines, tolls and ferries show here as they happen.',
    today: 'Today',
    yesterday: 'Yesterday',
    kinds: {
      delivered: 'Job delivered',
      cancelled: 'Job cancelled',
      fined: 'Fine',
      toll: 'Toll',
      ferry: 'Ferry',
      train: 'Train',
      fuel: 'Fuel station',
    },
    otherOut: 'Other expenses',
    otherIn: 'Other income',
    otherOutDetail: 'Fuel, repairs, purchases (seen at the save)',
    otherInDetail: 'Seen at the save',
    tollDetail: 'Toll gate',
    offences: OFFENCES.en,
    foot: 'Balance from the game’s latest save, plus what happened since. Fuel, repairs and purchases show at the next save.',
    converted: 'Converted from the game’s currency at a fixed, approximate rate.',
  },
  pt: {
    account: 'Conta',
    show: 'Mostrar saldo',
    hide: 'Esconder saldo',
    noSave: 'O saldo aparece assim que o jogo salvar (o salvamento automático também conta).',
    income: 'Entradas',
    spent: 'Saídas',
    history: 'Histórico',
    clear: 'Apagar',
    clearAsk: 'Apagar todo o histórico?',
    clearNote: 'O saldo continua como está. Não dá pra desfazer.',
    clearYes: 'Apagar histórico',
    cancel: 'Cancelar',
    empty: 'Nada ainda: entregas, multas, pedágios e balsas aparecem aqui na hora.',
    today: 'Hoje',
    yesterday: 'Ontem',
    kinds: {
      delivered: 'Frete entregue',
      cancelled: 'Frete cancelado',
      fined: 'Multa',
      toll: 'Pedágio',
      ferry: 'Balsa',
      train: 'Trem',
      fuel: 'Posto de combustível',
    },
    otherOut: 'Outros gastos',
    otherIn: 'Outras entradas',
    otherOutDetail: 'Diesel, oficina, compras (visto no save)',
    otherInDetail: 'Visto no save',
    tollDetail: 'Praça de pedágio',
    offences: OFFENCES.pt,
    foot: 'Saldo do último save do jogo, mais o que aconteceu depois. Diesel, oficina e compras aparecem no próximo save.',
    converted: 'Convertido da moeda do jogo por um câmbio fixo e aproximado.',
  },
}
