import { useEffect } from 'react'
import { postToHost } from '../shared/host'
import { FIT_LIMITS, isFitted, type ScreenFit, useShownFit } from '../cabin/state/screenFit'
import { List, Row } from './ios'

export interface ScreenFitWords {
  screenFit: string
  screenFitHint: string
  screenFitNone: string
  edgeTop: string
  edgeBottom: string
  edgeLeft: string
  edgeRight: string
  proportion: string
  fitSize: string
  restoreFit: string
}

/**
 * The cab's screen's Adjust Screen on the phone (owner 2026-11-02: "and if it opens so small I can't reach Settings
 * to adjust it?"): each setting a step down or up at a touch, sent to the cab's page through the host
 * (cabin/state/screenFit.ts), which tells back what it is now.
 */
export function ScreenFitList({ words }: { words: ScreenFitWords }) {
  const key = useShownFit((state) => state.key)
  const fit = useShownFit((state) => state.fit)
  // What the cab's is, opened: it tells it as it changes, perhaps before this page was.
  useEffect(() => postToHost({ type: 'screenFit' }), [])
  if (key === null) {
    return (
      <List header={words.screenFit}>
        <Row label={<span className="ios-row-off">{words.screenFitNone}</span>} />
      </List>
    )
  }
  const settings: [keyof ScreenFit, string, { min: number; max: number; step: number }][] = [
    ['top', words.edgeTop, FIT_LIMITS.edge],
    ['bottom', words.edgeBottom, FIT_LIMITS.edge],
    ['left', words.edgeLeft, FIT_LIMITS.edge],
    ['right', words.edgeRight, FIT_LIMITS.edge],
    ['stretch', words.proportion, FIT_LIMITS.stretch],
    ['size', words.fitSize, FIT_LIMITS.size],
  ]
  return (
    <List header={words.screenFit} footer={words.screenFitHint}>
      {settings.map(([name, label, limits]) => (
        <Row
          key={name}
          label={label}
          value={
            <Stepper
              label={label}
              value={fit[name]}
              limits={limits}
              onChange={(value) => postToHost({ type: 'screenFit', change: { [name]: value } })}
            />
          }
        />
      ))}
      <button
        type="button"
        className="ios-row ios-row-action"
        disabled={!isFitted(fit)}
        onClick={() => postToHost({ type: 'screenFit', reset: true })}
      >
        {words.restoreFit}
      </button>
    </List>
  )
}

/** A value with a minus and a plus beside it, a step each, within its limits. */
function Stepper({
  label,
  value,
  limits: { min, max, step },
  onChange,
}: {
  label: string
  value: number
  limits: { min: number; max: number; step: number }
  onChange: (value: number) => void
}) {
  const go = (by: number) => {
    const next = Math.min(max, Math.max(min, Number((Math.round((value - min) / step + by) * step + min).toFixed(4))))
    if (next !== value) onChange(next)
  }
  return (
    <span className="ios-stepper">
      <button type="button" aria-label={`${label} −`} disabled={value <= min} onClick={() => go(-1)}>
        −
      </button>
      <span className="ios-stepper-value">{`${Math.round(value * 1000) / 10}%`}</span>
      <button type="button" aria-label={`${label} +`} disabled={value >= max} onClick={() => go(1)}>
        +
      </button>
    </span>
  )
}
