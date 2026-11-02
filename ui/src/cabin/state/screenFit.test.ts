import { beforeEach, describe, expect, it } from 'vitest'
import type { TruckInfo } from '../../shared/host'
import { fitKey, fitStyle, isFitted, NO_FIT, readFit, useScreenFits } from './screenFit'

const bronco = { id: 'vehicle.ford.bronco_24' } as TruckInfo

describe('screen fit', () => {
  beforeEach(() => useScreenFits.setState({ fits: {} }))

  it('is kept for the screen and the vehicle on it', () => {
    expect(fitKey('ford-cars', bronco)).toBe('ford-cars|ford.bronco_24')
    expect(fitKey('generic', null)).toBe('generic|')
    expect(fitKey(null, bronco)).toBeNull()
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
      top: 0.2,
      stretch: 1.35,
      size: 0.6,
    })
    expect(readFit(null)).toEqual(NO_FIT)
  })

  it('forgets a fit set back to none', () => {
    const { change } = useScreenFits.getState()
    change('ford-cars|ford.bronco_24', { top: 0.05 })
    expect(isFitted(useScreenFits.getState().fits['ford-cars|ford.bronco_24'])).toBe(true)
    change('ford-cars|ford.bronco_24', { top: 0 })
    expect(useScreenFits.getState().fits).toEqual({})
  })
})
