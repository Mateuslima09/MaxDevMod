import { useCabin, useLive } from '../cabin/state/store'
import { type FloatScreenAction, postToHost } from '../shared/host'
import { List, Row, Segmented } from './ios'

/**
 * MaxDevMod's own screen floating in the cab (owner 2026-11-20: "a screen of our own that works in any truck, no
 * accessory"; crates/plugin float_screen.rs): switched on or off, and moved and turned live for the truck driven now
 * (kept for it). Moves are in the truck's own directions; 2 cm or 2° a press.
 */

export const FLOAT_SCREEN_WORDS = {
  en: {
    noGame: 'The game isn’t running',
    screen: 'Floating screen',
    on: 'On',
    off: 'Off',
    mapHeader: 'Multimedia map',
    mapLabel: 'Map shown',
    mapMaxDevMod: 'MaxDevMod',
    mapGameGps: 'Game GPS',
    mapNote:
      'Choose MaxDevMod for the built-in map (recommended on the floating screen) or Game GPS to show the truck’s original GPS through the dashboard.',
    status: 'In the cab now',
    seen: 'Showing',
    notSeen: 'Not seen (look from inside the cab)',
    oldPlugin: 'The game’s plugin is older: restart the game',
    move: 'Move',
    forward: 'Forward',
    back: 'Back',
    up: 'Up',
    down: 'Down',
    left: 'Left',
    right: 'Right',
    across: 'Sideways',
    height: 'Height',
    depth: 'Forward / back',
    turning: 'Turn',
    turn: 'Turn',
    tilt: 'Tilt',
    roll: 'Lean',
    size: 'Size',
    smaller: 'Smaller',
    bigger: 'Bigger',
    reset: 'Back to where it goes by itself',
    note:
      'Your own MaxDevMod screen, drawn in the cab of any truck (no accessory). Use it with the screen’s key and the mouse, as the truck’s screen. Its place is kept for each truck.',
    depthTip:
      'To make it smaller use Size, not Forward: pushed forward it goes behind the dashboard and slides as your head moves. Bring it to the dashboard’s surface: looking to the sides, it should stay on the same spot.',
  },
  pt: {
    noGame: 'O jogo não está aberto',
    screen: 'Tela flutuante',
    on: 'Ligada',
    off: 'Desligada',
    mapHeader: 'Mapa na multimídia',
    mapLabel: 'Qual mapa exibir',
    mapMaxDevMod: 'MaxDevMod',
    mapGameGps: 'GPS do jogo',
    mapNote:
      'Escolha MaxDevMod para exibir o mapa próprio (recomendado na tela flutuante) ou GPS do jogo para mostrar o GPS original do caminhão na tela do painel.',
    status: 'Na cabine agora',
    seen: 'Aparecendo',
    notSeen: 'Não aparece (olhe de dentro da cabine)',
    oldPlugin: 'O plugin do jogo é antigo: reinicie o jogo',
    move: 'Mover',
    forward: 'Frente',
    back: 'Trás',
    up: 'Subir',
    down: 'Descer',
    left: 'Esquerda',
    right: 'Direita',
    across: 'Lado',
    height: 'Altura',
    depth: 'Frente / trás',
    turning: 'Girar',
    turn: 'Girar',
    tilt: 'Inclinar',
    roll: 'Tombar',
    size: 'Tamanho',
    smaller: 'Menor',
    bigger: 'Maior',
    reset: 'Voltar ao lugar padrão',
    note:
      'Sua própria tela do MaxDevMod, desenhada na cabine de qualquer caminhão (sem acessório). Use com a tecla da tela e o mouse, como a tela do caminhão. O lugar fica guardado para cada caminhão.',
    depthTip:
      'Para diminuir, use Tamanho, não Frente: empurrada para a frente ela fica atrás do painel e escorrega quando a cabeça mexe. Traga até encostar no painel: olhando para os lados, ela deve ficar no mesmo ponto.',
  },
}

export type FloatScreenWords = (typeof FLOAT_SCREEN_WORDS)['en']

export function FloatScreenApp({ words, language }: { words: FloatScreenWords; language: 'en' | 'pt' }) {
  const t = useLive((state) => state.telemetry)
  const useGameMap = useCabin((state) => state.useGameMap)
  const setUseGameMap = useCabin((state) => state.setUseGameMap)

  const mapSelector = (
    <List header={words.mapHeader} footer={words.mapNote}>
      <Row label={words.mapLabel}>
        <Segmented
          options={[
            ['maxdevmod', words.mapMaxDevMod],
            ['game', words.mapGameGps],
          ]}
          chosen={useGameMap ? 'game' : 'maxdevmod'}
          choose={(value) => setUseGameMap(value === 'game')}
        />
      </Row>
    </List>
  )

  if (!t) {
    return (
      <>
        {mapSelector}
        <p className="ios-empty">{words.noGame}</p>
      </>
    )
  }
  const press = (action: FloatScreenAction) => postToHost({ type: 'floatScreen', action })
  const known = t.floatOn !== undefined
  const on = t.floatOn ?? false
  const [right, up, forward, turn, tilt, roll] = t.floatPose ?? [0, 0, 0, 0, 0, 0]
  const cm = (metres: number) => {
    const n = Math.round(metres * 100)
    return `${n > 0 ? '+' : n < 0 ? '−' : ''}${Math.abs(n)} cm`
  }
  const degrees = (value: number) => {
    const n = Math.round(value * 10) / 10
    const text = String(Math.abs(n))
    return `${n > 0 ? '+' : n < 0 ? '−' : ''}${language === 'pt' ? text.replace('.', ',') : text}°`
  }
  return (
    <>
      <List footer={words.note}>
        <Row label={words.screen}>
          <Segmented
            options={[
              ['on', words.on],
              ['off', words.off],
            ]}
            chosen={on ? 'on' : 'off'}
            choose={(value) => {
              if ((value === 'on') !== on) press('toggle')
            }}
          />
        </Row>
        <Row label={words.status} value={!known ? words.oldPlugin : t.floatSeen ? words.seen : words.notSeen} />
      </List>
      {mapSelector}
      {on && (
        <>
          <article className="ios-hero ios-camera-hero">
            <div className="ios-hero-kicker">{words.move}</div>
            <div className="ios-camera-pad">
              <button type="button" className="ios-camera-key up" onClick={() => press('up')}>
                <span aria-hidden>▲</span>
                {words.up}
              </button>
              <button type="button" className="ios-camera-key forward" onClick={() => press('left')}>
                <span aria-hidden>◀</span>
                {words.left}
              </button>
              <div className="ios-camera-now">
                <div>
                  {words.across} <b>{cm(right)}</b>
                </div>
                <div>
                  {words.height} <b>{cm(up)}</b>
                </div>
              </div>
              <button type="button" className="ios-camera-key back" onClick={() => press('right')}>
                {words.right}
                <span aria-hidden>▶</span>
              </button>
              <button type="button" className="ios-camera-key down" onClick={() => press('down')}>
                {words.down}
                <span aria-hidden>▼</span>
              </button>
            </div>
          </article>
          <List header={words.size} footer={words.depthTip}>
            <PairRow
              name={words.size}
              value=""
              less={{ label: words.smaller, press: () => press('smaller') }}
              more={{ label: words.bigger, press: () => press('bigger') }}
            />
          </List>
          <List header={words.turning}>
            <PairRow
              name={words.depth}
              value={cm(forward)}
              less={{ label: words.back, press: () => press('back') }}
              more={{ label: words.forward, press: () => press('forward') }}
            />
            <PairRow
              name={words.turn}
              value={degrees(turn)}
              less={{ label: words.left, press: () => press('turnLeft') }}
              more={{ label: words.right, press: () => press('turnRight') }}
            />
            <PairRow
              name={words.tilt}
              value={degrees(tilt)}
              less={{ label: words.down, press: () => press('tiltDown') }}
              more={{ label: words.up, press: () => press('tiltUp') }}
            />
            <PairRow
              name={words.roll}
              value={degrees(roll)}
              less={{ label: words.left, press: () => press('rollLeft') }}
              more={{ label: words.right, press: () => press('rollRight') }}
            />
          </List>
          <List>
            <button type="button" className="ios-row ios-row-action" onClick={() => press('reset')}>
              {words.reset}
            </button>
          </List>
        </>
      )}
    </>
  )
}

/** A move or turn: its name and how far, and a button each way (as the reversing camera's rows). */
function PairRow({
  name,
  value,
  less,
  more,
}: {
  name: string
  value: string
  less: { label: string; press: () => void }
  more: { label: string; press: () => void }
}) {
  return (
    <div className="ios-row ios-camera-row">
      <span className="ios-camera-name">
        {name} <b>{value}</b>
      </span>
      <span className="ios-camera-buttons">
        <button type="button" onClick={less.press}>
          {less.label}
        </button>
        <button type="button" onClick={more.press}>
          {more.label}
        </button>
      </span>
    </div>
  )
}
