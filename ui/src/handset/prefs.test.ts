import { describe, expect, it } from 'vitest'
import { distance, speed } from './prefs'

describe('distance', () => {
  it('says a short way in feet or metres, not 0', () => {
    expect(distance(250, 'mi')).toBe('820 ft')
    expect(distance(250, 'km')).toBe('250 m')
  })

  it('says a long way in the chosen unit', () => {
    expect(distance(625_000, 'mi')).toBe('388 mi')
    expect(distance(625_000, 'km')).toBe('625 km')
  })
})

describe('speed', () => {
  it('turns metres per second into mph or km/h', () => {
    expect(speed(25, 'km')).toBe(90)
    expect(speed(-25, 'mi')).toBe(56)
  })
})
