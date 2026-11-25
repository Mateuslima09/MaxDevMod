import { type ReactNode, useEffect, useState } from 'react'
import { type BankAnswer, postToHost, type WalletEntry } from '../shared/host'
import { useLive } from '../cabin/state/store'
import { refreshLiveBank, repayLiveLoan, takeLiveLoan } from './bankFleetBridge'
import { Segmented } from './ios'
import { OFFENCES } from './offences'
import { formatMoney, type Prefs, shownCurrency } from './prefs'

/**
 * Nupay (live in-memory bank mirror + instant loans):
 * - Mirrors `bank_u->money_account` live from memory so balance matches ETS2/ATS 1:1 in real time.
 * - Reads active `bank_loan_u`s with exact daily installments, paid amount, installments paid & remaining,
 *   and allows instant repayment without restarting.
 * - Reads live `bank_data_u->loan_offers` directly from the game's bank (matching ETS2/ATS & economy mods)
 *   and credits loans instantly into the player's account without restarting.
 */
export function NupayApp({ language, currency }: { language: 'en' | 'pt'; currency: Prefs['currency'] }) {
  const wallet = useLive((state) => state.wallet)
  const [hidden, setHidden] = useState(false)
  const [tab, setTab] = useState<'history' | 'loans'>('history')
  const [confirming, setConfirming] = useState(false)
  const [busyKey, setBusyKey] = useState<string | null>(null)
  const [confirmRepay, setConfirmRepay] = useState<number | null>(null)
  const [toast, setToast] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null)

  const words = WORDS[language]
  const from = wallet?.currency ?? 'USD'
  const shown = shownCurrency(currency, from)
  const money = (amount: number, signed = false) => formatMoney(amount, from, currency, language, signed)

  const entries = wallet?.entries ?? []
  const loans = wallet?.loans ?? []
  const offers = wallet?.offers ?? []
  const liveBank = wallet?.liveBank ?? false
  const bankUnlocked = wallet?.bankUnlocked ?? true
  const loanLimit = wallet?.loanLimit ?? 0
  const availableCredit = wallet?.availableCredit ?? 0

  useEffect(() => {
    void refreshLiveBank()
  }, [])

  useEffect(() => {
    if (!toast) return
    const t = window.setTimeout(() => setToast(null), 4200)
    return () => window.clearTimeout(t)
  }, [toast])

  const income = entries.filter((entry) => entry.amount > 0).reduce((sum, entry) => sum + entry.amount, 0)
  const spent = entries.filter((entry) => entry.amount < 0).reduce((sum, entry) => sum + entry.amount, 0)
  const totalDebt = loans.reduce((sum, loan) => sum + loan.amount, 0)
  const totalDaily = loans.reduce((sum, loan) => sum + loan.dailyPayment, 0)
  const creditUsedPct = loanLimit > 0 ? Math.min(100, Math.round((totalDebt / loanLimit) * 100)) : 0

  const handleTakeLoan = async (offerIndex: number, amount: number) => {
    if (busyKey) return
    setBusyKey(`take-${offerIndex}`)
    const ans = await takeLiveLoan(offerIndex)
    setBusyKey(null)
    if (ans === 'ok') {
      setToast({
        kind: 'ok',
        text:
          language === 'pt'
            ? `Empréstimo de ${money(amount)} creditado na hora!`
            : `Loan of ${money(amount)} credited immediately!`,
      })
    } else {
      setToast({ kind: 'err', text: bankErrorText(ans, language) })
    }
  }

  const handleRepayLoan = async (loanIndex: number) => {
    if (busyKey) return
    setBusyKey(`repay-${loanIndex}`)
    setConfirmRepay(null)
    const ans = await repayLiveLoan(loanIndex)
    setBusyKey(null)
    if (ans === 'ok') {
      setToast({
        kind: 'ok',
        text: language === 'pt' ? 'Financiamento quitado com sucesso!' : 'Loan repaid in full!',
      })
    } else {
      setToast({ kind: 'err', text: bankErrorText(ans, language) })
    }
  }

  return (
    <div className="nupay">
      <header className="nupay-head">
        <div className="nupay-head-row">
          <div className="nupay-hello">{words.account}</div>
          <span className={`nupay-live-pill${liveBank ? ' on' : ''}`}>
            <span className="nupay-live-dot" />
            {liveBank
              ? language === 'pt'
                ? 'Sincronizado'
                : 'Live sync'
              : language === 'pt'
                ? 'Offline'
                : 'Offline'}
          </span>
        </div>
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

      <div className="nupay-tabs">
        <Segmented
          options={[
            ['history', language === 'pt' ? 'Extrato' : 'Activity'],
            ['loans', language === 'pt' ? 'Financiamentos' : 'Loans'],
          ]}
          chosen={tab}
          choose={(next) => setTab(next as 'history' | 'loans')}
        />
      </div>

      {toast && (
        <div className={`nupay-toast ${toast.kind}`} role="status">
          {toast.text}
        </div>
      )}

      {tab === 'loans' ? (
        <div className="nupay-section">
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

          {liveBank && loanLimit > 0 && (
            <div className="nupay-credit-card">
              <div className="nupay-credit-head">
                <span>{language === 'pt' ? 'Crédito disponível' : 'Available credit'}</span>
                <strong>{hidden ? '••••' : money(availableCredit)}</strong>
              </div>
              <div className="nupay-loan-bar">
                <div className="nupay-loan-bar-fill credit" style={{ width: `${100 - creditUsedPct}%` }} />
              </div>
              <div className="nupay-credit-sub">
                <span>
                  {language === 'pt' ? 'Limite do banco' : 'Bank limit'}: {hidden ? '••••' : money(loanLimit)}
                </span>
                <span>{100 - creditUsedPct}%</span>
              </div>
            </div>
          )}

          {liveBank && !bankUnlocked && (
            <div className="nupay-pending" role="status">
              <div className="nupay-pending-info">
                <strong>{language === 'pt' ? 'Banco ainda bloqueado' : 'Bank not unlocked yet'}</strong>
                <span>
                  {language === 'pt'
                    ? 'Complete mais entregas para liberar financiamentos no banco do jogo.'
                    : 'Complete more deliveries to unlock bank loans in-game.'}
                </span>
              </div>
            </div>
          )}

          <div className="nupay-block">
            <div className="nupay-history-head">
              <h2 className="nupay-title">
                {language === 'pt' ? 'Financiamentos ativos' : 'Active loans'}
                {loans.length > 0 ? ` (${loans.length})` : ''}
              </h2>
            </div>
            {loans.length === 0 ? (
              <p className="nupay-empty">
                {language === 'pt' ? 'Nenhum financiamento ativo no momento.' : 'No active bank loans right now.'}
              </p>
            ) : (
              <div className="ios-list">
                {loans.map((loan, idx) => {
                  const loanIndex = loan.index ?? idx
                  const paidAmount = loan.paidAmount ?? Math.max(0, loan.originalAmount - loan.amount)
                  const daysTotal = loan.daysTotal ?? loan.duration
                  const daysRemaining = loan.daysRemaining ?? Math.max(0, daysTotal)
                  const daysPaid = loan.daysPaid ?? Math.max(0, daysTotal - daysRemaining)
                  const paidPct = Math.max(
                    0,
                    Math.min(100, Math.round((paidAmount / Math.max(1, loan.originalAmount)) * 100)),
                  )
                  const isBusy = busyKey === `repay-${loanIndex}`
                  const askRepay = confirmRepay === loanIndex

                  return (
                    <div key={`${loanIndex}-${loan.originalAmount}`} className="ios-row nupay-loan-card">
                      <div className="nupay-loan-top">
                        <div className="nupay-loan-main">
                          <span className="ios-row-label">{hidden ? '••••' : money(loan.amount)}</span>
                          <span className="nupay-loan-sub-label">
                            {language === 'pt' ? 'restante de ' : 'left of '}
                            {hidden ? '••••' : money(loan.originalAmount)}
                          </span>
                        </div>
                        <span className="nupay-loan-rate">
                          {language === 'pt'
                            ? `${money(loan.dailyPayment)}/dia · ${Math.round(loan.interestRate * 100)}%`
                            : `${money(loan.dailyPayment)}/day · ${Math.round(loan.interestRate * 100)}%`}
                        </span>
                      </div>

                      <div className="nupay-loan-bar">
                        <div className="nupay-loan-bar-fill" style={{ width: `${paidPct}%` }} />
                      </div>

                      <div className="nupay-loan-metrics">
                        <span>
                          {language === 'pt' ? 'Pago' : 'Paid'}: <strong>{hidden ? '••••' : money(paidAmount)}</strong> (
                          {paidPct}%)
                        </span>
                        <span>
                          {language === 'pt'
                            ? `Parcelas: ${daysPaid}/${daysTotal} · faltam ${daysRemaining}`
                            : `Installments: ${daysPaid}/${daysTotal} · ${daysRemaining} left`}
                        </span>
                      </div>

                      {liveBank && (
                        <div className="nupay-loan-actions">
                          {askRepay ? (
                            <div className="nupay-repay-confirm">
                              <span>
                                {language === 'pt'
                                  ? `Quitar ${money(loan.amount)} agora?`
                                  : `Pay off ${money(loan.amount)} now?`}
                              </span>
                              <div className="nupay-repay-btns">
                                <button type="button" onClick={() => setConfirmRepay(null)}>
                                  {words.cancel}
                                </button>
                                <button
                                  type="button"
                                  className="primary"
                                  disabled={isBusy}
                                  onClick={() => void handleRepayLoan(loanIndex)}
                                >
                                  {isBusy
                                    ? '...'
                                    : language === 'pt'
                                      ? 'Confirmar'
                                      : 'Confirm'}
                                </button>
                              </div>
                            </div>
                          ) : (
                            <button
                              type="button"
                              className="nupay-repay-btn"
                              disabled={!loan.canRepay || busyKey !== null}
                              onClick={() => setConfirmRepay(loanIndex)}
                            >
                              {isBusy
                                ? language === 'pt'
                                  ? 'Quitando...'
                                  : 'Repaying...'
                                : loan.canRepay
                                  ? language === 'pt'
                                    ? `Quitar agora (${money(loan.amount)})`
                                    : `Pay off now (${money(loan.amount)})`
                                  : language === 'pt'
                                    ? 'Saldo insuficiente p/ quitar'
                                    : 'Insufficient balance to pay off'}
                            </button>
                          )}
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            )}
          </div>

          <div className="nupay-block">
            <div className="nupay-history-head">
              <h2 className="nupay-title">
                {language === 'pt' ? 'Financiamentos do banco' : 'Available bank loans'}
              </h2>
            </div>
            {!liveBank ? (
              <p className="nupay-empty">
                {language === 'pt'
                  ? 'Abra o jogo (ETS2 ou ATS) para contratar financiamentos na hora sem reiniciar.'
                  : 'Launch ETS2 or ATS to take bank loans live without restarting.'}
              </p>
            ) : offers.length === 0 ? (
              <p className="nupay-empty">
                {language === 'pt'
                  ? 'Nenhuma oferta encontrada no banco do jogo.'
                  : 'No loan offers found in the game bank.'}
              </p>
            ) : (
              <div className="ios-list">
                {offers.map((offer) => {
                  const isBusy = busyKey === `take-${offer.index}`
                  const disabled = !offer.canTake || busyKey !== null
                  return (
                    <button
                      key={`${offer.index}-${offer.amount}-${offer.durationDays}`}
                      type="button"
                      disabled={disabled}
                      className={`ios-row ios-row-button nupay-offer${!offer.canTake ? ' locked' : ''}`}
                      onClick={() => void handleTakeLoan(offer.index, offer.amount)}
                    >
                      <span className="ios-row-text">
                        <span className="ios-row-label">{money(offer.amount)}</span>
                        <span className="ios-row-detail">
                          {language === 'pt'
                            ? `${money(offer.dailyPayment)}/dia · ${offer.durationDays} parcelas · ${Math.round(offer.interestRate * 100)}% juros`
                            : `${money(offer.dailyPayment)}/day · ${offer.durationDays} days · ${Math.round(offer.interestRate * 100)}% APR`}
                        </span>
                        <span className="ios-row-detail nupay-offer-total">
                          {language === 'pt' ? 'Total a pagar' : 'Total repayment'}: {money(offer.totalRepayment)}
                        </span>
                      </span>
                      <span className={`nupay-offer-pill${!offer.canTake ? ' disabled' : ''}`}>
                        {isBusy
                          ? '...'
                          : offer.canTake
                            ? language === 'pt'
                              ? 'Contratar'
                              : 'Take'
                            : language === 'pt'
                              ? 'Sem limite'
                              : 'Over limit'}
                      </span>
                    </button>
                  )
                })}
              </div>
            )}
          </div>

          <p className="nupay-foot">
            {language === 'pt'
              ? 'Conectado diretamente ao banco do jogo em tempo real: empréstimos e quitações entram na conta na hora, sem reiniciar.'
              : 'Connected live to the in-game bank: loans and repayments apply immediately without restarting.'}
          </p>
        </div>
      ) : (
        <div className="nupay-section">
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
                        <svg viewBox="0 0 24 24">{KIND_GLYPHS[entry.kind] ?? KIND_GLYPHS.other}</svg>
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
            {liveBank ? words.footLive : words.foot}
            {shown !== from && ` ${words.converted}`}
          </p>
        </div>
      )}
    </div>
  )
}

function bankErrorText(ans: BankAnswer, language: 'en' | 'pt'): string {
  const pt = language === 'pt'
  switch (ans) {
    case 'multiplayer':
      return pt ? 'Indisponível no TruckersMP.' : 'Unavailable in TruckersMP.'
    case 'locked':
      return pt ? 'Banco ainda bloqueado neste perfil.' : 'Bank is still locked on this profile.'
    case 'limitExceeded':
      return pt ? 'Limite de crédito insuficiente no banco.' : 'Not enough bank credit limit.'
    case 'insufficientFunds':
      return pt ? 'Saldo insuficiente para quitar este empréstimo.' : 'Insufficient balance to repay this loan.'
    case 'notReady':
    case 'noAnswer':
      return pt ? 'Jogo não está pronto. Entre no caminhão e tente novamente.' : 'Game not ready. Enter the world and try again.'
    default:
      return pt ? 'Não foi possível concluir a operação no banco.' : 'Could not complete the bank operation.'
  }
}

type Words = (typeof WORDS)['en']

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
  return words.kinds[entry.kind] ?? words.otherIn
}

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
  loan: (
    <path
      d="M3 10h18M5 10v8M9.5 10v8M14.5 10v8M19 10v8M2 20h20M12 3 3 8h18Z"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinejoin="round"
    />
  ),
  loanRepay: (
    <path
      d="M4 12.5 9.5 18 20 6.5"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.4"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
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
    noSave: 'Enter the game to sync your live bank balance.',
    income: 'Money in',
    spent: 'Money out',
    history: 'History',
    clear: 'Clear',
    clearAsk: 'Clear the whole history?',
    clearNote: 'The balance stays as it is. This can’t be undone.',
    clearYes: 'Clear history',
    cancel: 'Cancel',
    empty: 'Nothing yet: deliveries, fines, tolls, loans and ferries show here as they happen.',
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
      loan: 'Bank loan',
      loanRepay: 'Loan paid off',
    },
    otherOut: 'Other expenses',
    otherIn: 'Other income',
    otherOutDetail: 'Fuel, repairs, purchases (seen at the save)',
    otherInDetail: 'Seen at the save',
    tollDetail: 'Toll gate',
    offences: OFFENCES.en,
    foot: 'Balance from the game’s latest save, plus what happened since.',
    footLive: 'Live balance synced directly from the game’s bank in real time.',
    converted: 'Converted from the game’s currency at a fixed, approximate rate.',
  },
  pt: {
    account: 'Conta Nupay',
    show: 'Mostrar saldo',
    hide: 'Esconder saldo',
    noSave: 'Entre no jogo para sincronizar o saldo real em tempo real.',
    income: 'Entradas',
    spent: 'Saídas',
    history: 'Histórico',
    clear: 'Apagar',
    clearAsk: 'Apagar todo o histórico?',
    clearNote: 'O saldo continua como está. Não dá pra desfazer.',
    clearYes: 'Apagar histórico',
    cancel: 'Cancelar',
    empty: 'Nada ainda: entregas, empréstimos, multas, pedágios e balsas aparecem aqui na hora.',
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
      loan: 'Empréstimo bancário',
      loanRepay: 'Quitação de financiamento',
    },
    otherOut: 'Outros gastos',
    otherIn: 'Outras entradas',
    otherOutDetail: 'Diesel, oficina, compras (visto no save)',
    otherInDetail: 'Visto no save',
    tollDetail: 'Praça de pedágio',
    offences: OFFENCES.pt,
    foot: 'Saldo do último save do jogo, mais o que aconteceu depois.',
    footLive: 'Saldo sincronizado ao vivo diretamente com a conta bancária do jogo.',
    converted: 'Convertido da moeda do jogo por um câmbio fixo e aproximado.',
  },
}
