import { describe, expect, it } from 'vitest'
import { INK_ENOUGH, isSigned, PAPER_WORDS, paperNumber, tonnes } from './paperWords'

describe("the cargo's paper", () => {
  it('has every word in English and Portuguese', () => {
    for (const language of ['en', 'pt'] as const) {
      for (const [name, word] of Object.entries(PAPER_WORDS[language])) {
        expect(word, `${language}.${name}`).toBeTruthy()
      }
    }
    expect(Object.keys(PAPER_WORDS.pt).sort()).toEqual(Object.keys(PAPER_WORDS.en).sort())
  })

  it('has the same number for the same job, six digits', () => {
    const job = { cargo: 'Soja', sourceCity: 'Cuiabá', destinationCity: 'Santos', deliveryTime: 123456 }
    expect(paperNumber(job)).toMatch(/^\d{6}$/)
    expect(paperNumber(job)).toBe(paperNumber({ ...job }))
    expect(paperNumber({ ...job, destinationCity: 'Paranaguá' })).not.toBe(paperNumber(job))
  })

  it('writes the weight in tonnes as the language does', () => {
    expect(tonnes(18500, 'pt')).toBe('18,5 t')
    expect(tonnes(18500, 'en')).toBe('18.5 t')
    expect(tonnes(22000, 'pt')).toBe('22 t')
  })

  it('takes a signature only once enough is drawn', () => {
    expect(isSigned(0)).toBe(false)
    expect(isSigned(INK_ENOUGH - 1)).toBe(false)
    expect(isSigned(INK_ENOUGH)).toBe(true)
  })
})
