import { useContext } from 'react'
import { setWestplayOpen } from '../useHostStatus'
import { Group, Note, Panel, Row, Status, Switch, Tick } from '../room/parts'
import { banner, WordsContext } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import type { GameInfo } from '../setup'
import type { ScreenResolution, Split } from '../useSettings'
import { type PageProps, screenState } from './page'

/**
 * The truck's screen: where MaxDevMod is now (its headline), MaxDevMod on it or the truck's own, and the
 * connection from MaxDevMod to the game to the screen. On the room's screen, Home, and MaxDevMod saying the
 * same in a banner.
 */
export function ScreenPage({ status, settings, setup, cw }: PageProps) {
  const w = cw.screen
  const state = screenState(status)
  const games = gamesSetUp(setup, w.game)
  // Waiting for a game: every one set up named, in the words that name American Truck Simulator.
  const names = games.map((game) => game.name).join(' / ')
  const named = (text: string) => text.replace('American Truck Simulator', names)
  // Known once the cabin page has said (it owns the state): the switch waits for it.
  const open = status?.westplayOpen ?? null
  useScreenAt(
    state === 'original' ? 'dashboard' : 'home',
    GLOW.home,
    state === 'live'
      ? banner(w.banners.live)
      : state === 'original'
        ? banner(w.banners.original)
        : state === 'unsupported'
          ? banner(w.banners.unsupported)
          : state === 'notInCab'
            ? banner(w.banners.notInCab)
            : state === 'noGame'
              ? banner({ ...w.banners.noGame, text: named(w.banners.noGame.text) }, true)
              : null,
  )
  const fps = status?.framesPerSecond ?? 0
  return (
    <Panel place="page" title={w.titles[state]} sub={named(w.subs[state])}>
      <Group place={4}>
        <Row
          title={w.show}
          detail={w.showDetail}
          accessory={
            <Switch
              on={open ?? false}
              disabled={open === null || !status?.screenDrawing}
              label={w.show}
              change={(on) => {
                setWestplayOpen(on).catch((error: unknown) => console.error('MaxDevMod could not be switched', error))
              }}
            />
          }
        />
      </Group>
      <Group title={w.connection} place={5}>
        <Row
          title={w.engine}
          value={status?.error ? cw.values.failed : status ? w.engineRunning : w.engineStarting}
          accessory={<Status tone={status?.error ? 'bad' : status ? 'good' : 'busy'} />}
        />
        {games.map((game) => {
          const open = status?.gameConnected === true && status.game === game.kind
          return (
            <Row
              key={game.kind}
              title={game.name}
              value={open ? w.gameConnected : w.gameNotRunning}
              accessory={<Status tone={open ? 'good' : 'idle'} />}
            />
          )
        })}
        <Row
          title={w.truckScreen}
          value={
            state === 'unsupported'
              ? w.screenUnsupported
              : status?.screenDrawing
                ? w.screenLive.replace('{fps}', String(fps))
                : w.screenNotInView
          }
          accessory={
            <Status
              tone={
                state === 'unsupported'
                  ? 'idle'
                  : status?.screenDrawing
                    ? 'good'
                    : status?.gameConnected
                      ? 'busy'
                      : 'idle'
              }
            />
          }
        />
      </Group>
      {status?.gameElevated && (
        <Note tone="bad">
          <b>{w.elevated.title}</b> {w.elevated.text}
        </Note>
      )}
      <PhoneCode status={status} language={setup.language} />
      <KeepScreenOnToggle settings={settings} />
      <ResolutionChooser settings={settings} running={status?.gameConnected === true} />
      <SplitChooser settings={settings} running={status?.gameConnected === true} />
      <GpuChooser status={status} settings={settings} cw={cw} />
      {status?.error && <Note tone="bad">{status.error}</Note>}
    </Panel>
  )
}

/**
 * Keep the truck's multimedia screen powered on even when the truck's ignition/electrics are turned off.
 */
function KeepScreenOnToggle({ settings: state }: { settings: PageProps['settings'] }) {
  const { settings, update } = state
  const { code } = useContext(WordsContext)
  if (!settings) return null
  const w = KEEP_ON_WORDS[code.startsWith('pt') ? 'pt' : code.startsWith('tr') ? 'tr' : 'en']
  const on = settings.screen.keepScreenOn === true
  return (
    <Group title={w.section} place={5}>
      <Row
        title={w.title}
        detail={w.detail}
        accessory={
          <Switch
            on={on}
            label={w.title}
            change={(keepScreenOn) => void update({ ...settings, screen: { ...settings.screen, keepScreenOn } })}
          />
        }
      />
    </Group>
  )
}

const KEEP_ON_WORDS = {
  en: {
    section: 'Power (Truck Off)',
    title: 'Keep screen on with truck off',
    detail: 'Keep the multimedia screen running and interactive even when the truck’s ignition and electrics are off (also available on the phone with Alt+4).',
  },
  pt: {
    section: 'Energia (Caminhão Desligado)',
    title: 'Manter tela ligada com o caminhão desligado',
    detail: 'Mantém a central multimídia funcionando mesmo com o caminhão e a elétrica desligados (também pode ligar/desligar no painel ou pelo celular Alt+4).',
  },
  tr: {
    section: 'Güç (Motor Kapalı)',
    title: 'Kamyon kapalıyken ekranı açık tut',
    detail: 'Kamyon ve elektrik kapalıyken bile multimedya ekranının çalışmaya devam etmesini sağlar (Alt+4 telefondan da açıp kapatabilirsiniz).',
  },
} as const

/**
 * Render-target resolution & sharpness (`gps_drawable_size`) for all ETS2/ATS & mod screens, plus guidance on live
 * manual screen fitting (`Ajustes › Ajustar Tela` or `Alt+4` phone) for unmapped modded trucks.
 */
function ResolutionChooser({ settings: state, running }: { settings: PageProps['settings']; running: boolean }) {
  const { settings, update } = state
  const { code } = useContext(WordsContext)
  if (!settings) return null
  const w = RESOLUTION_WORDS[code.startsWith('pt') ? 'pt' : code.startsWith('tr') ? 'tr' : 'en']
  const chosen: ScreenResolution = settings.screen.resolution ?? 'high'
  const choose = (resolution: ScreenResolution) => {
    if (resolution !== chosen) void update({ ...settings, screen: { ...settings.screen, resolution } })
  }
  return (
    <>
      <Group title={w.title} place={6}>
        {(['high', 'ultra', 'standard', 'gameDefault'] as const).map((tier) => (
          <Row
            key={tier}
            title={w[tier].title}
            detail={w[tier].detail}
            accessory={chosen === tier ? <Tick key={tier} /> : undefined}
            onClick={() => choose(tier)}
          />
        ))}
      </Group>
      <Note tone="calm">{running ? `${w.note} ${w.restart}` : w.note}</Note>
    </>
  )
}

const RESOLUTION_WORDS: Record<
  'en' | 'pt' | 'tr',
  {
    title: string
    high: { title: string; detail: string }
    ultra: { title: string; detail: string }
    standard: { title: string; detail: string }
    gameDefault: { title: string; detail: string }
    note: string
    restart: string
  }
> = {
  en: {
    title: 'Screen resolution & sharpness (Live)',
    high: { title: 'High · HD (1280×720)', detail: 'Recommended: native 1:1 sharpness across every truck' },
    ultra: { title: 'Ultra · Max sharpness (1280×720+)', detail: 'Native HD with contrast sharpening for small text' },
    standard: { title: 'Standard · Balanced (640×360)', detail: 'Lighter on older graphics cards' },
    gameDefault: { title: 'Game default', detail: 'Keeps the game’s or mod’s original screen target' },
    note: 'Applies immediately on the truck screen without restarting the game — also available live in Settings › Adjust Screen inside the cabin.',
    restart: 'Live change active — switch tiers anytime while driving.',
  },
  pt: {
    title: 'Resolução e nitidez da tela (Ao Vivo)',
    high: { title: 'Alta · HD (1280×720)', detail: 'Recomendada: nitidez nativa 1:1 em qualquer caminhão' },
    ultra: { title: 'Ultra · Nitidez máxima (1280×720+)', detail: 'HD nativo com realce de nitidez para textos pequenos' },
    standard: { title: 'Padrão · Equilibrada (640×360)', detail: 'Mais leve para placas de vídeo antigas' },
    gameDefault: { title: 'Original do jogo', detail: 'Mantém a resolução nativa original do jogo ou do mod' },
    note: 'Aplica na hora direto no painel do caminhão, sem precisar reiniciar o jogo — também disponível ao vivo em Ajustes › Ajustar Tela dentro do painel.',
    restart: 'Troca ao vivo ativa — você pode mudar enquanto dirige.',
  },
  tr: {
    title: 'Ekran çözünürlüğü ve netliği (Anlık)',
    high: { title: 'Yüksek · HD (1280×720)', detail: 'Önerilen: her araçta doğal 1:1 netlik' },
    ultra: { title: 'Ultra · Maksimum netlik (1280×720+)', detail: 'Küçük yazılar için netleştirme ile doğal HD' },
    standard: { title: 'Standart · Dengeli (640×360)', detail: 'Eski ekran kartları için daha hafif' },
    gameDefault: { title: 'Oyun varsayılanı', detail: 'Oyunun veya modun orijinal ekran çözünürlüğünü korur' },
    note: 'Oyunu yeniden başlatmadan anında kamyon ekranına uygulanır — kabin içinde Ayarlar › Ekranı Ayarla menüsünde de bulunur.',
    restart: 'Anlık değişim aktif — sürerken istediğiniz zaman değiştirin.',
  },
}

/**
 * The split screen (owner 2026-11-02): the game's own map squeezed into one half of the truck's screen, MaxDevMod in the
 * other, or off, the game's own screen as it is. MaxDevMod's mod is made again for it once the game is closed, and the
 * game reads it as it starts; the cab's sidebar then switches MaxDevMod between its half and the whole screen. Its words
 * in English, Portuguese and Turkish for now, as the phone's codes.
 */
function SplitChooser({ settings: state, running }: { settings: PageProps['settings']; running: boolean }) {
  const { settings, update } = state
  const { code } = useContext(WordsContext)
  if (!settings) return null
  const w = SPLIT_WORDS[code.startsWith('pt') ? 'pt' : code.startsWith('tr') ? 'tr' : 'en']
  const chosen = settings.screen.split ?? 'off'
  const choose = (split: Split) => {
    if (split !== chosen) void update({ ...settings, screen: { ...settings.screen, split } })
  }
  return (
    <>
      <Group title={w.title} place={6}>
        {(['off', 'mapLeft', 'mapRight'] as const).map((split) => (
          <Row
            key={split}
            title={w[split]}
            accessory={chosen === split ? <Tick key={split} /> : undefined}
            onClick={() => choose(split)}
          />
        ))}
      </Group>
      <Note tone="calm">{running ? `${w.note} ${w.restart}` : w.note}</Note>
    </>
  )
}

const SPLIT_WORDS: Record<
  'en' | 'pt' | 'tr',
  { title: string; off: string; mapLeft: string; mapRight: string; note: string; restart: string }
> = {
  en: {
    title: 'Split screen',
    off: 'Off: the game’s screen as it is',
    mapLeft: 'The game’s map on the left',
    mapRight: 'The game’s map on the right',
    note: 'The game’s map squeezed into one half of the truck’s screen, not stretched, and MaxDevMod in the other: the split screen button in MaxDevMod’s sidebar switches between that and MaxDevMod on the whole screen. Changed here, it takes the game’s next start.',
    restart: 'Restart the game for it.',
  },
  pt: {
    title: 'Tela dividida',
    off: 'Desligada: a tela do jogo como ela é',
    mapLeft: 'Mapa do jogo à esquerda',
    mapRight: 'Mapa do jogo à direita',
    note: 'O mapa do jogo fica espremido numa metade da tela do caminhão, sem esticar, e o MaxDevMod na outra: o botão de tela dividida na barra lateral do MaxDevMod troca entre isso e o MaxDevMod na tela inteira. Mudando aqui, vale quando o jogo abrir de novo.',
    restart: 'Reinicie o jogo para aplicar.',
  },
  tr: {
    title: 'Bölünmüş ekran',
    off: 'Kapalı: oyunun ekranı olduğu gibi',
    mapLeft: 'Oyunun haritası solda',
    mapRight: 'Oyunun haritası sağda',
    note: 'Oyunun haritası tırın ekranının bir yarısına sığdırılır, uzatılmadan; MaxDevMod diğer yarıda. MaxDevMod’in kenar çubuğundaki bölünmüş ekran düğmesi bununla tam ekran MaxDevMod arasında geçiş yapar. Burada değişince oyunun bir sonraki açılışında geçerli olur.',
    restart: 'Uygulamak için oyunu yeniden başlatın.',
  },
}

/**
 * The graphics card MaxDevMod's screen is drawn on: MaxDevMod's pick (the game's), or one the player chooses
 * when that one stutters, as iOS lists choices (a tick on the chosen, as the onboarding's game folders). The
 * one drawing it now says so; a choice starts MaxDevMod's screen again on that card. Only with more than one
 * card to choose from.
 */
function GpuChooser({ status, settings: state, cw }: Pick<PageProps, 'status' | 'settings' | 'cw'>) {
  const w = cw.screen
  const { settings, update } = state
  const adapters = status?.adapters ?? []
  if (!settings || adapters.length < 2) return null
  const chosen = adapters.some((adapter) => adapter.id === settings.graphics.gpu) ? settings.graphics.gpu : null
  const choose = (gpu: string | null) => {
    if (gpu !== settings.graphics.gpu) void update({ ...settings, graphics: { gpu } })
  }
  return (
    <>
      <Group title={cw.about.gpu} place={6}>
        <Row
          title={w.gpuAuto}
          detail={w.gpuAutoDetail}
          accessory={chosen === null ? <Tick key="auto" /> : undefined}
          onClick={() => choose(null)}
        />
        {adapters.map((adapter) => (
          <Row
            key={adapter.id}
            title={adapter.name}
            detail={adapter.name === status?.cabAdapter ? w.gpuInUse : undefined}
            accessory={chosen === adapter.id ? <Tick key={adapter.id} /> : undefined}
            onClick={() => choose(adapter.id)}
          />
        ))}
      </Group>
      <Note tone="calm">{w.gpuNote}</Note>
    </>
  )
}

/**
 * The player's phone as a second screen (crates/host remote.rs, owner 2026-11-01): the QR codes that open it, on
 * the PC's network: the truck's panel (CarPlay), and the phone in the game alone. Its words in English,
 * Portuguese and Turkish for now (the rest English), not yet in the companion's own words: those are every
 * language's at once.
 */
function PhoneCode({ status, language }: { status: PageProps['status']; language: string | null }) {
  if (!status?.phoneUrl || !status.phoneQr) return null
  const w = PHONE_WORDS[language?.startsWith('pt') ? 'pt' : language?.startsWith('tr') ? 'tr' : 'en']
  return (
    <>
      <Group title={w.title} place={6}>
        <div className="phone-codes ob-in">
          <QrCode svg={status.phoneQr} url={status.phoneUrl} caption={w.panel} />
          {status.handsetUrl && status.handsetQr && (
            <QrCode svg={status.handsetQr} url={status.handsetUrl} caption={w.handset} />
          )}
          {status.wheelUrl && status.wheelQr && (
            <QrCode svg={status.wheelQr} url={status.wheelUrl} caption={w.wheel} />
          )}
        </div>
      </Group>
      <Note tone="calm">{w.text}</Note>
      {status.wheelUrl && <Note tone="calm">{w.wheelNote}</Note>}
    </>
  )
}

function QrCode({ svg, url, caption }: { svg: string; url: string; caption: string }) {
  return (
    <figure className="phone-code">
      <img src={`data:image/svg+xml;utf8,${encodeURIComponent(svg)}`} alt={url} />
      <figcaption>{caption}</figcaption>
    </figure>
  )
}

const PHONE_WORDS: Record<
  'en' | 'pt' | 'tr',
  { title: string; text: string; panel: string; handset: string; wheel: string; wheelNote: string }
> = {
  en: {
    title: 'Your phone',
    text: 'Scan one with your phone’s camera, on the same Wi-Fi as this PC: the truck’s panel (CarPlay) on your phone, or the phone in the game (Alt+4) on your phone. Both work at the same time as the game’s.',
    panel: 'Truck panel',
    handset: 'Phone in the game',
    wheel: 'Steering wheel',
    wheelNote:
      'Steering wheel: your phone steers the truck with its gyroscope, with throttle and brake on the screen. It opens over HTTPS: the first time, the phone’s browser warns about the certificate; choose Advanced and go on (it’s this PC’s own).',
  },
  pt: {
    title: 'Seu celular',
    text: 'Escaneie um deles com a câmera do celular, na mesma rede Wi-Fi deste PC: o painel do caminhão (CarPlay) no celular, ou o celular do jogo (Alt+4) no celular. Os dois funcionam junto com o do jogo.',
    panel: 'Painel do caminhão',
    handset: 'Celular do jogo',
    wheel: 'Volante',
    wheelNote:
      'Volante: o celular vira o volante do caminhão pelo giroscópio, com acelerador e freio na tela. Ele abre em HTTPS: na primeira vez o navegador do celular avisa do certificado; toque em Avançado e continue (é o certificado deste PC).',
  },
  tr: {
    title: 'Telefonunuz',
    text: 'Bu bilgisayarla aynı Wi-Fi ağındayken birini telefonunuzun kamerasıyla okutun: tırın paneli (CarPlay) ya da oyundaki telefon (Alt+4), telefonunuzda.',
    panel: 'Tır paneli',
    handset: 'Oyundaki telefon',
    wheel: 'Direksiyon',
    wheelNote:
      'Direksiyon: telefonunuz jiroskopuyla tırı yönlendirir, gaz ve fren ekranda. HTTPS ile açılır: ilk seferde tarayıcı sertifika uyarısı verir; Gelişmiş’e dokunup devam edin (bu bilgisayarın kendi sertifikası).',
  },
}

/**
 * The games MaxDevMod is set up in, each a row of the connection (owner 2026-11-02: both games there, the one open
 * green, to tell which is): American Truck Simulator first. None set up yet: the game, by the words' name.
 */
function gamesSetUp(setup: PageProps['setup'], name: string): { kind: GameInfo['kind']; name: string }[] {
  const games = [setup.game, ...setup.others].filter((game): game is GameInfo => game !== null)
  if (games.length === 0) return [{ kind: 'ats', name }]
  const unique = games.filter((game, at) => games.findIndex((other) => other.kind === game.kind) === at)
  return unique.sort((a, b) => (a.kind === b.kind ? 0 : a.kind === 'ats' ? -1 : 1))
}
