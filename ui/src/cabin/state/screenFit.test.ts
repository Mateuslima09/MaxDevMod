import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PageMessage, TruckInfo } from '../../shared/host'
import { applyPhoneFit, fitKey, fitStyle, isFitted, NO_FIT, readFit, useScreenFits } from './screenFit'

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
    expect(readFit({ top: 0.9, bottom: -0.5, stretch: 3, size: 0.1, left: 'x' })).toEqual({
      ...NO_FIT,
      top: 0.45,
      bottom: -0.25,
      stretch: 2,
      size: 0.5,
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
