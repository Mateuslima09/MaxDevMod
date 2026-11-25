import { describe, expect, it } from 'vitest'
import type { GameNotification, GameNotificationKind } from '../shared/host'
import { isWarning, notificationApp, notificationText } from './notificationText'

const KINDS: GameNotificationKind[] = [
  'payAttendant',
  'delivered',
  'cancelled',
  'fined',
  'toll',
  'ferry',
  'train',
  'fuel',
  'otherIn',
  'otherOut',
  'fuelLow',
  'damage',
  'crash',
  'repaired',
  'rest',
  'restNow',
  'jobNew',
  'jobDeadline',
  'jobLate',
  'brakeHot',
  'brakeGone',
]

const game = (change: Partial<GameNotification>): GameNotification => ({
  app: 'nupay',
  kind: 'delivered',
  currency: 'USD',
  ...change,
})

describe("the game's notifications", () => {
  it('say each kind in both languages', () => {
    for (const kind of KINDS) {
      for (const language of ['en', 'pt'] as const) {
        const text = notificationText(game({ kind, amount: -100, value: 30, detail: 'engine' }), language)
        expect(text, `${language}.${kind}`).toBeTruthy()
        expect(text).not.toContain('undefined')
      }
    }
  })

  it('say a payment in the currency chosen', () => {
    const paid = game({ amount: 21469, detail: 'Tomates → Reno' })
    expect(notificationText(paid, 'en', 'USD')).toBe('Payment received: $21,469 for the job · Tomates → Reno')
    expect(notificationText(paid, 'pt', 'auto')).toMatch(/^Pix recebido: US\$\s?21\.469 pelo frete · Tomates → Reno$/)
    // An expense says its sum without a minus: "paid" says it went out.
    expect(notificationText(game({ kind: 'toll', amount: -22, count: 2 }), 'en', 'USD')).toBe('Toll paid: $22 (×2)')
  })

  it('say a fine by its offence and the damage by its part', () => {
    expect(notificationText(game({ kind: 'fined', amount: -300, detail: 'speeding' }), 'pt', 'USD')).toContain(
      'Excesso de velocidade',
    )
    expect(notificationText(game({ app: 'truck', kind: 'damage', value: 25, detail: 'engine' }), 'pt')).toBe(
      'Caminhão com 25% de dano (motor). Passe na oficina.',
    )
  })

  it('say the fuel owed in the pump’s money', () => {
    const owed = game({ kind: 'payAttendant', total: 130.5, detail: 'R$' })
    expect(notificationText(owed, 'pt')).toBe(
      'Pague R$ 130,50 ao frentista: ele está na frente do caminhão, aperte E perto dele',
    )
  })

  it('name their app, and warn of what goes wrong', () => {
    expect(notificationApp(game({}), 'pt')).toBe('Nupay')
    expect(notificationApp(game({ app: 'truck' }), 'pt')).toBe('Caminhão')
    expect(notificationApp(game({ app: 'freight' }), 'en')).toBe('Jobs')
    expect(isWarning(game({ kind: 'fined' }))).toBe(true)
    expect(isWarning(game({ kind: 'delivered' }))).toBe(false)
    expect(isWarning({ app: 'tiktok', kind: 'gift', user: 'Ana' })).toBe(false)
  })
})
