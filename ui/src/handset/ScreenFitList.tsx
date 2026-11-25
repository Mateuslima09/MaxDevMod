import { useEffect } from 'react'
import { postToHost } from '../shared/host'
import {
  activeFitPreset,
  FIT_LIMITS,
  isFitted,
  SCREEN_FIT_PRESETS,
  type ScreenFit,
  type ScreenFitPresetId,
  useShownFit,
} from '../cabin/state/screenFit'
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
  fitPresets?: Record<ScreenFitPresetId, string> & { title: string }
}

const DEFAULT_PRESET_LABELS: Record<ScreenFitPresetId, string> & { title: string } = {
  title: 'Formato rápido (Manual)',
  auto: 'Automático',
  widescreen: '16:9 Widescreen',
  sixteenTen: '16:10 Médio',
  classic: '4:3 Clássico',
  ultrawide: '21:9 Ultrawide',
  bandCenter: 'Faixa Central (Mod)',
}

/**
 * The cab's screen's Adjust Screen on the phone: quick manual format presets for unmapped modded trucks plus fine
 * edge/proportion/size steppers, sent live to the cab's page through the host.
 */
export function ScreenFitList({ words }: { words: ScreenFitWords }) {
  const key = useShownFit((state) => state.key)
  const fit = useShownFit((state) => state.fit)
  useEffect(() => postToHost({ type: 'screenFit' }), [])
  if (key === null) {
    return (
      <List header={words.screenFit}>
        <Row label={<span className="ios-row-off">{words.screenFitNone}</span>} />
      </List>
    )
  }
  const presets = words.fitPresets ?? DEFAULT_PRESET_LABELS
  const activePreset = activeFitPreset(fit)
  const settings: [keyof ScreenFit, string, { min: number; max: number; step: number }][] = [
    ['top', words.edgeTop, FIT_LIMITS.edge],
    ['bottom', words.edgeBottom, FIT_LIMITS.edge],
    ['left', words.edgeLeft, FIT_LIMITS.edge],
    ['right', words.edgeRight, FIT_LIMITS.edge],
    ['stretch', words.proportion, FIT_LIMITS.stretch],
    ['size', words.fitSize, FIT_LIMITS.size],
  ]
  return (
    <>
      <List header={presets.title}>
        {SCREEN_FIT_PRESETS.map((preset) => {
          const on = activePreset === preset.id
          return (
            <button
              key={preset.id}
              type="button"
              className="ios-row ios-row-button"
              aria-pressed={on}
              onClick={() =>
                preset.id === 'auto'
                  ? postToHost({ type: 'screenFit', reset: true })
                  : postToHost({ type: 'screenFit', change: preset.fit })
              }
            >
              <span className="ios-row-text">
                <span className="ios-row-label">{presets[preset.id]}</span>
              </span>
              {on && (
                <span className="ios-row-value" style={{ color: 'var(--ios-blue)', fontWeight: 600 }}>
                  ✓
                </span>
              )}
            </button>
          )
        })}
      </List>
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
    </>
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
