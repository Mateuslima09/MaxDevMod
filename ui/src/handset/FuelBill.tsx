import { useEffect, useState } from 'react'
import { useLive } from '../cabin/state/store'
import { type FuelBill as Bill, postToHost } from '../shared/host'
import { convert, type Currency } from './prefs'

/** How long "Paid" shows before the phone goes back (ms). */
const PAID_FOR = 1300

/** The money a pump's sign stands for. */
function currencyOf(sign: string): Currency {
  if (sign.includes('R$')) return 'BRL'
  if (sign.includes('€')) return 'EUR'
  return 'USD'
}

/** A sum as the language writes it, with the pump's sign. */
function money(value: number, sign: string, language: 'en' | 'pt'): string {
  const number = value.toLocaleString(language === 'pt' ? 'pt-BR' : 'en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
  return `${sign} ${number}`
}

/**
 * The fuel filled by hand, paid at the pump's attendant on the phone (owner 2026-11-12: "not charged at once: go to
 * the attendant and pay, on the phone"): the player walked up to them and asked to pay (the plugin's `refuel`), and
 * the host shows this large, with the mouse, as Nupay's payment: the fuel, how much, its price, the total, and Pay.
 * Paid, it's in Nupay's history (in the game's money, at the phone's fixed rates); put down, the bill stays open.
 */
export function FuelBill({ bill, language }: { bill: Bill; language: 'en' | 'pt' }) {
  const words = WORDS[language]
  const gameCurrency = useLive((state) => state.wallet?.currency ?? 'USD')
  const [paid, setPaid] = useState(false)
  const unit = bill.gallons ? words.gallons : words.litres
  const amount = bill.amount.toLocaleString(language === 'pt' ? 'pt-BR' : 'en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })

  useEffect(() => {
    if (!paid) return
    const timer = setTimeout(() => {
      // In the game's money, an expense: the pump's total from its own money at the phone's rates.
      const game = Math.round(-convert(bill.total, currencyOf(bill.sign), gameCurrency))
      postToHost({ type: 'fuelPaid', id: bill.id, amount: game, detail: `${words.diesel} · ${amount} ${unit}` })
    }, PAID_FOR)
    return () => clearTimeout(timer)
  }, [paid, bill, gameCurrency, amount, unit, words.diesel])

  return (
    <article className="fuel-bill">
      <header className="fuel-bill-head">
        <span className="fuel-bill-app">Nupay</span>
        {!paid && (
          <button
            type="button"
            className="fuel-bill-close"
            aria-label={words.close}
            onClick={() => postToHost({ type: 'fuelPutDown', id: bill.id })}
          >
            ×
          </button>
        )}
        <p>{words.charge}</p>
        <strong className="fuel-bill-total">{money(bill.total, bill.sign, language)}</strong>
      </header>
      <section className="fuel-bill-body">
        <div className="fuel-bill-station">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 3h8a1 1 0 0 1 1 1v7h1a2 2 0 0 1 2 2v4a1 1 0 0 0 2 0V9l-2-2 1.4-1.4L21 8.2V17a3 3 0 0 1-6 0v-4h-1v8H4V4a1 1 0 0 1 1-1Zm1 2v5h6V5Z" />
          </svg>
          <div>
            <b>{words.station}</b>
            <span>{words.diesel}</span>
          </div>
        </div>
        <dl className="fuel-bill-rows">
          <dt>{unit}</dt>
          <dd>{amount}</dd>
          <dt>{words.price}</dt>
          <dd>{money(bill.price, bill.sign, language)}</dd>
          <dt>{words.total}</dt>
          <dd className="strong">{money(bill.total, bill.sign, language)}</dd>
        </dl>
        {paid ? (
          <div className="fuel-bill-paid" role="status">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 16.2 4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4Z" />
            </svg>
            {words.paid}
          </div>
        ) : (
          <button type="button" className="fuel-bill-pay" onClick={() => setPaid(true)}>
            {words.pay} {money(bill.total, bill.sign, language)}
          </button>
        )}
        <p className="fuel-bill-note">{words.note}</p>
      </section>
    </article>
  )
}

const WORDS = {
  en: {
    close: 'Not now',
    charge: 'Payment to',
    station: 'Fuel station',
    diesel: 'Diesel',
    litres: 'Litres',
    gallons: 'Gallons',
    price: 'Price per unit',
    total: 'Total',
    pay: 'Pay',
    paid: 'Paid',
    note: 'Pay the attendant before leaving the pump.',
  },
  pt: {
    close: 'Agora não',
    charge: 'Pagamento para Posto',
    station: 'Posto de combustível',
    diesel: 'Diesel',
    litres: 'Litros',
    gallons: 'Galões',
    price: 'Preço por litro',
    total: 'Total',
    pay: 'Pagar',
    paid: 'Pago',
    note: 'Pague o frentista antes de sair da bomba.',
  },
}
