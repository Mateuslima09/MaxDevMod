import { describe, expect, it } from 'vitest'
import type { TikTokNotification } from '../shared/host'
import type { AppId } from './icons'
import { notificationText, waitingWith } from './notificationText'
import { TIKTOK_WORDS } from './tiktokWords'

describe('TikTok app', () => {
  it('has every word in English and Portuguese', () => {
    for (const language of ['en', 'pt'] as const) {
      for (const [name, word] of Object.entries(TIKTOK_WORDS[language])) {
        expect(word, `${language}.${name}`).toBeTruthy()
      }
    }
    expect(Object.keys(TIKTOK_WORDS.pt).sort()).toEqual(Object.keys(TIKTOK_WORDS.en).sort())
  })

  it('includes tiktok in AppId', () => {
    const appId: AppId = 'tiktok'
    expect(appId).toBe('tiktok')
  })
})

describe("the phone's notifications", () => {
  const notification = (change: Partial<TikTokNotification>): TikTokNotification => ({
    app: 'tiktok',
    kind: 'follow',
    user: 'Mateus',
    ...change,
  })

  it('say who followed, in the phone’s language', () => {
    expect(notificationText(notification({}), 'pt')).toBe('Mateus seguiu você')
    expect(notificationText(notification({}), 'en')).toBe('Mateus followed you')
  })

  it('say a gift by its name and how many', () => {
    const gift = notification({ kind: 'gift', user: 'Ana', detail: 'Rose', count: 5 })
    expect(notificationText(gift, 'pt')).toBe('Ana enviou Rose ×5')
    expect(notificationText(gift, 'en')).toBe('Ana sent Rose ×5')
    // One of it, or a gift whose name didn't come.
    expect(notificationText(notification({ kind: 'gift', detail: 'Rose', count: 1 }), 'pt')).toBe('Mateus enviou Rose')
    expect(notificationText(notification({ kind: 'gift' }), 'pt')).toBe('Mateus enviou um presente')
    expect(notificationText(notification({ kind: 'gift' }), 'en')).toBe('Mateus sent a gift')
  })

  it('say a share and a subscription', () => {
    expect(notificationText(notification({ kind: 'share' }), 'pt')).toBe('Mateus compartilhou sua LIVE')
    expect(notificationText(notification({ kind: 'subscribe' }), 'pt')).toBe('Mateus se inscreveu')
    expect(notificationText(notification({ kind: 'share' }), 'en')).toBe('Mateus shared your LIVE')
    expect(notificationText(notification({ kind: 'subscribe' }), 'en')).toBe('Mateus subscribed')
  })

  it('wait their turn, the oldest let go on a busy LIVE', () => {
    let waiting: number[] = []
    for (let each = 1; each <= 12; each++) waiting = waitingWith(waiting, each)
    expect(waiting).toEqual([5, 6, 7, 8, 9, 10, 11, 12])
  })
})
