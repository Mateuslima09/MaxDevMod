import { useEffect, useState } from 'react'
import { Button, Checking, Group, Note, Panel, Row, Status, Tick } from '../room/parts'
import { banner, explain, gameSummary, useGameRunning, useWords } from '../room/room'
import { GLOW, useScreen, useScreenAt } from '../room/screen'
import { checkGame, type InstallReport, installIntoGame } from '../setup'
import { type PageProps, useGameCheck } from './page'
import { companionWords } from './words'

/**
 * Where WestPlay is installed and how to put it right: the game it's in, the plugin as it is
 * there, and a reinstall (the onboarding's install, again, while the game is closed). On the room's
 * screen, the check's scan while the game is looked at, and WestPlay booting again after a reinstall.
 */
export function Setup(props: PageProps) {
  const { setup, cw, check, checked, uninstall, tutorial } = props
  const screen = useScreen()
  const words = useWords()
  const w = cw.setup
  const error = useGameCheck(props)
  const running = useGameRunning()
  const [installing, setInstalling] = useState(false)
  const [report, setReport] = useState<InstallReport | null>(null)
  const [installError, setInstallError] = useState<string | null>(null)
  const game = setup.game

  useEffect(() => {
    screen.scan(!check && !error)
    return () => screen.scan(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [check, error])
  useScreenAt(
    'home',
    check ? GLOW.home : GLOW.scan,
    installing
      ? banner({ ...words.install.banners.installing, text: words.install.plugin }, true)
      : report?.done
        ? banner({ title: w.reinstalled, text: words.install.pluginDetail })
        : check
          ? banner(w.checked)
          : error
            ? null
            : banner(w.checking, true),
  )

  const reinstall = async (administrator: boolean) => {
    if (!game) return
    setInstalling(true)
    setInstallError(null)
    try {
      const result = await installIntoGame(game.dir, administrator)
      setReport(result)
      if (result.done) {
        screen.reboot()
        // Checked again: the plugin and the mod as they are now.
        checkGame(game.dir).then(checked, () => {})
      }
    } catch (e) {
      setInstallError(explain(words, e))
    } finally {
      setInstalling(false)
    }
  }

  const plugin = report?.plugin
  const asAdministrator = plugin?.state === 'needsAdministrator' || plugin?.state === 'declined'
  const pluginState = check?.plugin
  return (
    <Panel
      place="page"
      title={w.title}
      sub={w.sub}
      action={
        game && (
          <Button glass onClick={() => reinstall(asAdministrator)} busy={installing} disabled={running !== false}>
            {asAdministrator ? words.install.asAdministrator : w.reinstall}
          </Button>
        )
      }
    >
      {game && (
        <Group title={w.game} place={4}>
          <Row title={w.folder} detail={game.dir} />
          <Row
            title={words.check.version}
            detail={gameSummary(words, game)}
            accessory={game.tested ? <Tick done /> : undefined}
          />
        </Group>
      )}
      <Group title={w.installed} place={5}>
        <Row
          title={w.plugin}
          detail={words.install.pluginDetail}
          value={pluginState ? w.pluginStates[pluginState] : undefined}
          accessory={
            !check && !error ? (
              <Checking />
            ) : pluginState === 'current' ? (
              <Tick done />
            ) : (
              <Status tone={pluginState === 'missing' ? 'bad' : 'idle'} />
            )
          }
        />
      </Group>
      {check?.modProblem && (
        <Note tone="bad">
          <b>{words.check.cantInstall}</b> {check.modProblem}
        </Note>
      )}
      {check?.otherApps.includes('ymnOne') && (
        <Note tone="bad">
          <b>{words.check.otherCarPlay.title}</b> {words.check.otherCarPlay.text}
        </Note>
      )}
      {running && (
        <Note>
          <b>{w.closeGame.title}</b> {w.closeGame.text}
        </Note>
      )}
      {plugin?.state === 'needsAdministrator' && (
        <Note tone="calm">
          <b>{words.install.needsAdministrator.title}</b> {words.install.needsAdministrator.text}
        </Note>
      )}
      {plugin?.state === 'declined' && (
        <Note>
          <b>{words.install.declined.title}</b> {words.install.declined.text}
        </Note>
      )}
      {plugin?.state === 'inUse' && (
        <Note>
          <b>{words.install.inUse.title}</b> {words.install.inUse.text}
        </Note>
      )}
      {plugin?.state === 'failed' && (
        <Note tone="bad">
          <b>{words.install.pluginFailed}</b> {explain(words, plugin.error)}
        </Note>
      )}
      {report?.modError && (
        <Note tone="bad">
          <b>{words.install.modFailed}</b> {explain(words, report.modError)}
        </Note>
      )}
      {(installError ?? error) && <Note tone="bad">{installError ?? error}</Note>}
      <OtherGames props={props} running={running !== false} />
      <Group place={6}>
        <Row title={w.tutorial} link onClick={tutorial} />
        <Row title={cw.uninstall.row} danger onClick={uninstall} />
      </Group>
    </Panel>
  )
}

/**
 * The other games found (owner 2026-11-02: WestPlay in ETS2 as in ATS): each one set up ticked, the others with an
 * install (the onboarding's, into that game, while it's closed). Installed, WestPlay works with it until the other one
 * is played again; the companion reads what's set up anew.
 */
function OtherGames({ props, running }: { props: PageProps; running: boolean }) {
  const { setup, cw } = props
  const words = useWords()
  const w = cw.setup.otherGames ?? companionWords('en_us').setup.otherGames
  const [installing, setInstalling] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const current = setup.game?.kind
  const games = setup.found.filter((found) => found.kind !== current)
  if (!w || games.length === 0) return null
  const install = async (dir: string) => {
    setInstalling(dir)
    setError(null)
    try {
      const result = await installIntoGame(dir, false)
      if (result.done) location.reload()
      else setError(result.modError ?? (result.plugin.state === 'failed' ? result.plugin.error : result.plugin.state))
    } catch (e) {
      setError(explain(words, e))
    } finally {
      setInstalling(null)
    }
  }
  return (
    <>
      <Group title={w.title} place={6}>
        {games.map((game) => {
          const setUp = setup.others.some((other) => other.dir === game.dir)
          return (
            <Row
              key={game.dir}
              title={game.name}
              detail={gameSummary(words, game)}
              value={setUp ? w.setUp : undefined}
              accessory={
                setUp ? (
                  <Tick done />
                ) : (
                  <Button glass onClick={() => install(game.dir)} busy={installing === game.dir} disabled={running}>
                    {w.install}
                  </Button>
                )
              }
            />
          )
        })}
      </Group>
      <Note>{w.footer}</Note>
      {error && <Note tone="bad">{explain(words, error)}</Note>}
    </>
  )
}
