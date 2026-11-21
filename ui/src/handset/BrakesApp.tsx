import { useBrakes, useCharacterSwitches } from '../cabin/state/store'
import { type BrakeFadeLevel, type BrakeHudCorner, type BrakeHudSize, postToHost } from '../shared/host'
import { List, Row, Segmented } from './ios'

/**
 * Brake fade (owner 2026-11-21, host brake_fade.rs): the truck's and the trailer's discs live, how much brake is left,
 * and its options: on or off, how soon, the sounds' volumes and the brakes' HUD in the game.
 */

export const BRAKES_WORDS = {
  en: {
    noGame: 'The game isn’t running',
    truck: 'Truck',
    trailer: 'Trailer',
    noTrailer: 'No trailer',
    strength: 'Brake left',
    fade: 'Brake fade',
    on: 'On',
    off: 'Off',
    level: 'Fades from',
    light: 'Light',
    realistic: 'Realistic',
    hard: 'Hard',
    levelNote:
      'The brakes heat up as you use them and, overheated, lose force until they cool (faster at speed, in the rain and at night). Realistic: weaker from 250 °C, no brake left at 400 °C. Light: 100 °C later. Hard: 50 °C sooner. Down long hills use the engine brake and the retarder.',
    sounds: 'Sounds',
    squeal: 'Grinding (no brakes)',
    warning: 'Dashboard warning',
    mute: 'Off',
    low: 'Low',
    mid: 'Mid',
    high: 'High',
    hud: 'Brakes in the game',
    hudShown: 'Show',
    corner: 'Corner',
    bottomLeft: '↙',
    bottomRight: '↘',
    topLeft: '↖',
    topRight: '↗',
    size: 'Size',
    small: 'S',
    medium: 'M',
    large: 'L',
  },
  pt: {
    noGame: 'O jogo não está aberto',
    truck: 'Cavalo',
    trailer: 'Carreta',
    noTrailer: 'Sem carreta',
    strength: 'Freio que sobrou',
    fade: 'Fadiga do freio',
    on: 'Ligada',
    off: 'Desligada',
    level: 'Começa',
    light: 'Leve',
    realistic: 'Realista',
    hard: 'Dura',
    levelNote:
      'Os freios esquentam com o uso e, superaquecidos, perdem força até esfriar (mais rápido em velocidade, na chuva e à noite). Realista: enfraquece a partir de 250 °C, sem freio aos 400 °C. Leve: 100 °C depois. Dura: 50 °C antes. Na serra, use o freio motor e o retarder.',
    sounds: 'Sons',
    squeal: 'Rangido (sem freio)',
    warning: 'Aviso no painel',
    mute: 'Desl.',
    low: 'Baixo',
    mid: 'Médio',
    high: 'Alto',
    hud: 'Freios no jogo',
    hudShown: 'Mostrar',
    corner: 'Canto',
    bottomLeft: '↙',
    bottomRight: '↘',
    topLeft: '↖',
    topRight: '↗',
    size: 'Tamanho',
    small: 'P',
    medium: 'M',
    large: 'G',
  },
}

export type BrakesWords = (typeof BRAKES_WORDS)['en']

/** A disc's colour by its temperature: blue cold, green, yellow, orange, red (as the game's HUD draws it). */
export function discColour(celsius: number): string {
  if (celsius < 100) return '#3a8dff'
  if (celsius < 230) return '#2ecc71'
  if (celsius < 300) return '#f5c400'
  if (celsius < 380) return '#ff7a00'
  return '#d71920'
}

const VOLUMES: [string, number][] = [
  ['0', 0],
  ['30', 30],
  ['70', 70],
  ['100', 100],
]

/** A volume as the nearest of the four steps. */
function step(volume: number): string {
  return VOLUMES.reduce((best, [key, v]) => (Math.abs(v - volume) < Math.abs(Number(best) - volume) ? key : best), '70')
}

function Disc({ name, celsius }: { name: string; celsius: number | null }) {
  const colour = celsius === null ? '#8e8e93' : discColour(celsius)
  return (
    <div className="ios-brake-disc">
      <svg viewBox="0 0 64 64" width="64" height="64" aria-hidden>
        <circle cx="32" cy="32" r="28" fill={colour} />
        <circle cx="32" cy="32" r="10" fill="#1c1c1e" />
        {[0, 60, 120, 180, 240, 300].map((a) => (
          <circle
            key={a}
            cx={32 + 19 * Math.cos((a * Math.PI) / 180)}
            cy={32 + 19 * Math.sin((a * Math.PI) / 180)}
            r="2.6"
            fill="#1c1c1e"
          />
        ))}
        <path d="M46 10a26 26 0 0 1 8 10l-6 3a20 20 0 0 0-6-8Z" fill="#2c2c2e" />
      </svg>
      <div>{name}</div>
      <b>{celsius === null ? '—' : `${Math.round(celsius)} °C`}</b>
    </div>
  )
}

export function BrakesApp({ words }: { words: BrakesWords }) {
  const brakes = useBrakes()
  const o = useCharacterSwitches()
  const volumes = VOLUMES.map(
    ([key]) =>
      [
        key,
        ({ '0': words.mute, '30': words.low, '70': words.mid, '100': words.high } as Record<string, string>)[key],
      ] as [string, string],
  )
  return (
    <>
      {brakes.truck === null ? (
        <p className="ios-empty">{words.noGame}</p>
      ) : (
        <article className="ios-hero">
          <div className="ios-brake-discs">
            <Disc name={words.truck} celsius={brakes.truck} />
            <Disc name={brakes.trailer === null ? words.noTrailer : words.trailer} celsius={brakes.trailer} />
          </div>
          <div className="ios-hero-kicker">
            {words.strength}: <b>{brakes.strength ?? 100}%</b>
          </div>
        </article>
      )}
      <List footer={words.levelNote}>
        <Row label={words.fade}>
          <Segmented
            options={[
              ['on', words.on],
              ['off', words.off],
            ]}
            chosen={o.brakeFade ? 'on' : 'off'}
            choose={(value) => postToHost({ type: 'brakeFade', on: value === 'on' })}
          />
        </Row>
        <Row label={words.level}>
          <Segmented<BrakeFadeLevel>
            options={[
              ['light', words.light],
              ['realistic', words.realistic],
              ['hard', words.hard],
            ]}
            chosen={o.brakeFadeLevel}
            choose={(which) => postToHost({ type: 'brakeFadeLevel', which })}
          />
        </Row>
      </List>
      <List header={words.sounds}>
        <Row label={words.squeal}>
          <Segmented
            options={volumes}
            chosen={step(o.brakeSquealVolume)}
            choose={(key) => postToHost({ type: 'brakeSqueal', volume: Number(key) })}
          />
        </Row>
        <Row label={words.warning}>
          <Segmented
            options={volumes}
            chosen={step(o.brakeWarningVolume)}
            choose={(key) => postToHost({ type: 'brakeWarning', volume: Number(key) })}
          />
        </Row>
      </List>
      <List header={words.hud}>
        <Row label={words.hudShown}>
          <Segmented
            options={[
              ['on', words.on],
              ['off', words.off],
            ]}
            chosen={o.brakeHud ? 'on' : 'off'}
            choose={(value) => postToHost({ type: 'brakeHud', on: value === 'on' })}
          />
        </Row>
        <Row label={words.corner}>
          <Segmented<BrakeHudCorner>
            options={[
              ['topLeft', words.topLeft],
              ['topRight', words.topRight],
              ['bottomLeft', words.bottomLeft],
              ['bottomRight', words.bottomRight],
            ]}
            chosen={o.brakeHudCorner}
            choose={(which) => postToHost({ type: 'brakeHudCorner', which })}
          />
        </Row>
        <Row label={words.size}>
          <Segmented<BrakeHudSize>
            options={[
              ['small', words.small],
              ['medium', words.medium],
              ['large', words.large],
            ]}
            chosen={o.brakeHudSize}
            choose={(which) => postToHost({ type: 'brakeHudSize', which })}
          />
        </Row>
      </List>
    </>
  )
}
