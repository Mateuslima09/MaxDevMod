import { setWestplayOpen } from '../useHostStatus'
import { Group, Note, Panel, Row, Status, Switch, Tick } from '../room/parts'
import { banner } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import type { GameInfo } from '../setup'
import { type PageProps, screenState } from './page'

/**
 * The truck's screen: where WestPlay is now (its headline), WestPlay on it or the truck's own, and the
 * connection from WestPlay to the game to the screen. On the room's screen, Home, and WestPlay saying the
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
                setWestplayOpen(on).catch((error: unknown) => console.error('WestPlay could not be switched', error))
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
      <GpuChooser status={status} settings={settings} cw={cw} />
      {status?.error && <Note tone="bad">{status.error}</Note>}
    </Panel>
  )
}

/**
 * The graphics card WestPlay's screen is drawn on: WestPlay's pick (the game's), or one the player chooses
 * when that one stutters, as iOS lists choices (a tick on the chosen, as the onboarding's game folders). The
 * one drawing it now says so; a choice starts WestPlay's screen again on that card. Only with more than one
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
        </div>
      </Group>
      <Note tone="calm">{w.text}</Note>
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

const PHONE_WORDS: Record<'en' | 'pt' | 'tr', { title: string; text: string; panel: string; handset: string }> = {
  en: {
    title: 'Your phone',
    text: 'Scan one with your phone’s camera, on the same Wi-Fi as this PC: the truck’s panel (CarPlay) on your phone, or the phone in the game (Alt+4) on your phone. Both work at the same time as the game’s.',
    panel: 'Truck panel',
    handset: 'Phone in the game',
  },
  pt: {
    title: 'Seu celular',
    text: 'Escaneie um deles com a câmera do celular, na mesma rede Wi-Fi deste PC: o painel do caminhão (CarPlay) no celular, ou o celular do jogo (Alt+4) no celular. Os dois funcionam junto com o do jogo.',
    panel: 'Painel do caminhão',
    handset: 'Celular do jogo',
  },
  tr: {
    title: 'Telefonunuz',
    text: 'Bu bilgisayarla aynı Wi-Fi ağındayken birini telefonunuzun kamerasıyla okutun: tırın paneli (CarPlay) ya da oyundaki telefon (Alt+4), telefonunuzda.',
    panel: 'Tır paneli',
    handset: 'Oyundaki telefon',
  },
}

/**
 * The games WestPlay is set up in, each a row of the connection (owner 2026-11-02: both games there, the one open
 * green, to tell which is): American Truck Simulator first. None set up yet: the game, by the words' name.
 */
function gamesSetUp(setup: PageProps['setup'], name: string): { kind: GameInfo['kind']; name: string }[] {
  const games = [setup.game, ...setup.others].filter((game): game is GameInfo => game !== null)
  if (games.length === 0) return [{ kind: 'ats', name }]
  const unique = games.filter((game, at) => games.findIndex((other) => other.kind === game.kind) === at)
  return unique.sort((a, b) => (a.kind === b.kind ? 0 : a.kind === 'ats' ? -1 : 1))
}
