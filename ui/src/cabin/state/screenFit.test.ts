import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PageMessage, TruckInfo } from '../../shared/host'
import {
  applyPhoneFit,
  baseFit,
  edgeFrom,
  edgeLimits,
  fitKey,
  fitStyle,
  GENERIC_FIT,
  isFitted,
  NO_FIT,
  readFit,
  useScreenFits,
} from './screenFit'

const bronco = { id: 'vehicle.ford.bronco_24' } as TruckInfo

describe('screen fit', () => {
  beforeEach(() => useScreenFits.setState({ fits: {} }))

  it('is kept for the screen and the vehicle on it', () => {
    expect(fitKey('ford-cars', bronco)).toBe('ford-cars|ford.bronco_24')
    expect(fitKey('generic', null)).toBe('generic|')
    expect(fitKey(null, bronco)).toBeNull()
    // A mod screen's, whichever slot setup gave it.
    expect(fitKey('mod-3', bronco)).toBe(fitKey('mod-7', bronco))
  })

  it('leaves the page where it is without margins or proportion', () => {
    expect(fitStyle(NO_FIT)).toBeNull()
    // The size alone changes what's on it, not where the page is.
    expect(fitStyle({ ...NO_FIT, size: 1.2 })).toBeNull()
  })

  it('puts the page inside its margins, laid out wider for a narrower proportion', () => {
    const style = fitStyle({ ...NO_FIT, left: 0.05, right: 0.05, top: 0.1, stretch: 0.9 })
    expect(style).toEqual({
      left: '5vw',
      top: '10vh',
      width: '100vw',
      height: '90vh',
      transform: 'scaleX(0.9)',
    })
  })

  it('keeps each setting within its limits', () => {
    expect(readFit({ top: 0.9, stretch: 3, size: 0.1, left: 'x' })).toEqual({
      ...NO_FIT,
      top: 0.35,
      stretch: 1.35,
      size: 0.6,
    })
    expect(readFit(null)).toEqual(NO_FIT)
  })

  it("takes a phone's change for the cab's screen, and its reset, telling the phones", () => {
    const posted: PageMessage[] = []
    vi.stubGlobal('window', { chrome: { webview: { postMessage: (message: PageMessage) => posted.push(message) } } })
    applyPhoneFit('ford-cars|ford.bronco_24', { left: 0.04, stretch: 0.92 }, false)
    const fit = { ...NO_FIT, left: 0.04, stretch: 0.92 }
    expect(useScreenFits.getState().fits['ford-cars|ford.bronco_24']).toEqual(fit)
    expect(posted).toEqual([{ type: 'screenFitShown', key: 'ford-cars|ford.bronco_24', fit }])
    applyPhoneFit('ford-cars|ford.bronco_24', undefined, true)
    expect(useScreenFits.getState().fits).toEqual({})
    // No screen yet: nothing to fit.
    applyPhoneFit(null, { top: 0.1 }, false)
    expect(useScreenFits.getState().fits).toEqual({})
    vi.unstubAllGlobals()
  })

  it('forgets a fit set back to none', () => {
    const { change } = useScreenFits.getState()
    change('ford-cars|ford.bronco_24', { top: 0.05 })
    expect(isFitted(useScreenFits.getState().fits['ford-cars|ford.bronco_24'])).toBe(true)
    change('ford-cars|ford.bronco_24', { top: 0 })
    expect(useScreenFits.getState().fits).toEqual({})
  })
})

describe('the stock screen', () => {
  it('starts on the 7" Scania’s part, its edges going either way from there', () => {
    const key = 'generic|mercedes.actros2014'
    expect(baseFit(key)).toEqual(GENERIC_FIT)
    expect(baseFit('daf-2021|daf.2021')).toEqual(NO_FIT)
    const fits = useScreenFits.getState()
    fits.change(key, { bottom: 0.1 })
    expect(useScreenFits.getState().fits[key]).toEqual({ ...GENERIC_FIT, bottom: 0.1 })
    // Back to where it starts: nothing kept.
    fits.change(key, { bottom: GENERIC_FIT.bottom })
    expect(useScreenFits.getState().fits[key]).toBeUndefined()
    // All the way out: kept, the whole square.
    fits.change(key, { top: 0, bottom: 0, left: 0, right: 0 })
    expect(useScreenFits.getState().fits[key]).toEqual(NO_FIT)
    fits.reset(key)
  })
})

describe('an edge as the player adjusts it', () => {
  it('goes negative from where the screen starts, as far as its image goes', () => {
    expect(edgeLimits(GENERIC_FIT, 'bottom')).toEqual({ min: -0.265, max: 0.085, step: 0.005 })
    expect(edgeFrom(GENERIC_FIT, 'bottom', -0.2)).toBe(0.065)
    // A screen WestPlay is on whole: nothing to grow into.
    expect(edgeLimits(NO_FIT, 'top').min).toBe(-0)
  })
})
