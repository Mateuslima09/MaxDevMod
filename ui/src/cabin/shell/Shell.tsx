import { type CSSProperties, type ReactNode, useEffect, useLayoutEffect, useRef } from 'react'
import { hasVideoUnder, isInHost } from '../../shared/host'
import { useLook, useLookChanges, useOemStrip, useScreenFit, useScreenProfile, useSplitShown } from '../state/hooks'
import { announceFit, fitStyle, NO_FIT, type ScreenFit } from '../state/screenFit'
import { usePreparedIcons } from '../state/looks'
import { frameInsets, NO_INSETS, OEM_STRIPS } from '../state/oem'
import { RETURN_BUTTON_SIZE, returnButtonCentre, SCREEN_HEIGHT } from '../state/screen'
import { textScale } from '../state/textSize'
import { Player } from '../apps/youtube/Player'
import { type Hole, useYouTube } from '../apps/youtube/youtube'
import { useYouTubeMusic } from '../apps/youtubeMusic/youtubeMusic'
import { Keyboard } from '../keyboard/Keyboard'
import { useCabin, useLive } from '../state/store'
import { Sidebar } from './Sidebar'
import { useSidebarPeek } from './sidebarPeek'
import { VolumeHud } from './VolumeHud'
import './transitions.css'
import { Wallpaper } from './Wallpaper'

interface ShellProps {
  /** The current screen, placed in the CarPlay area. */
  children: ReactNode
  /** Full-screen content above everything, the OEM strip included (the boot sequence). */
  overlay?: ReactNode
  /** The first moments after the boot sequence: the screen fades in. */
  entering?: boolean
  /**
   * On the player's phone (phone/PhonePanel.tsx): the panel's height (in the cab screen's units), which the CarPlay
   * area is drawn to whole, and what its split screen shows beside the screen (phone/PhoneSplit.tsx): no game's map
   * under it there, nor the truck's screen to fit.
   */
  phone?: { height: number; beside: ReactNode }
}

/**
 * The frame every screen lives in: the CarPlay area (wallpaper, sidebar and the current screen) and, in the
 * Glass style, room for the truck's own OEM strip beside it. Everything outside the CarPlay area is
 * transparent: the game draws the truck's real strip there, underneath.
 */
/** How much smaller WestPlay is drawn in its half of the split screen. */
const SPLIT_SCALE = 0.7

export function Shell({ children, overlay, entering = false, phone }: ShellProps) {
  useLookChanges()
  usePreparedIcons()
  const { appearance, wallpaperSrc } = useLook()
  const profile = useScreenProfile()
  const { key: fitKey, fit } = useScreenFit()
  const handedOver = useCabin((state) => state.truckScreen)
  // The split screen, shown: WestPlay in its half, the game's map (squeezed into the other by the mod) beside it. On
  // the phone, nothing under the page: its own split, a second app beside the screen (owner 2026-11-03: the half
  // was black, the sidebar gone with it).
  const cabHalved = useSplitShown()
  const phoneSplit = useCabin((state) => state.splitShown && !state.truckScreen) && phone !== undefined
  const halved = phone ? null : cabHalved
  // YouTube's picture under the page, in full or in small: nothing the page paints may cover it.
  const videoShown = useYouTube((state) => state.open || state.hole !== null)
  // The cab's fit is the truck's screen's, not the phone's (owner 2026-11-03: it cut the panel's edges off there).
  useFittedRoot(phone ? NO_FIT : fit, handedOver || videoShown, halved)
  // With the game's map on the left, the sidebar on WestPlay's outer edge, the right (shell.css `.sidebar-right`): not in
  // the middle of the screen, against the map (owner 2026-11-02).
  useLayoutEffect(() => {
    document.documentElement.classList.toggle('sidebar-right', halved === 'mapLeft')
  }, [halved])
  // The phones' Adjust Screen shows what this screen's is (state/screenFit.ts).
  useEffect(() => announceFit(fitKey, fit), [fitKey, fit])
  const stripId = useOemStrip()
  const strip = stripId && OEM_STRIPS[stripId]
  const layout = useLive((state) => state.screenLayout)
  const insets = strip ? frameInsets(strip.insets, layout) : NO_INSETS
  const style = {
    '--oem-left': `${insets.left}px`,
    '--oem-bottom': `${insets.bottom}px`,
    // In its half (the split screen), smaller: as much of it fits as on a screen of its own. On the phone, drawn to
    // its height whole: a phone wider than the cab's screen has less of its height, where the sidebar didn't fit.
    '--ui-scale':
      (phone ? Math.min(1, phone.height / SCREEN_HEIGHT) : profile.scale * fit.size) *
      (halved || phoneSplit ? SPLIT_SCALE : 1),
  } as CSSProperties
  // The screen changes' snapshots hang off <html>, not the shell: they're cut to the CarPlay area with these
  // too (transitions.css).
  useLayoutEffect(() => {
    const root = document.documentElement.style
    root.setProperty('--oem-left', `${insets.left}px`)
    root.setProperty('--oem-bottom', `${insets.bottom}px`)
  }, [insets.left, insets.bottom])
  // The profile's margin round CarPlay, on <html> too: where the screens start (`--content-left`) is worked out
  // there from it. None: the stylesheet's own.
  useLayoutEffect(() => {
    const root = document.documentElement.style
    if (profile.edge === undefined) root.removeProperty('--edge')
    else root.setProperty('--edge', `${profile.edge}px`)
  }, [profile.edge])
  // The apps' text size (state/textSize.ts), on <html> so menus portalled out of the apps and the screen
  // changes' snapshots take it too.
  const textSize = useCabin((state) => state.textSize)
  useLayoutEffect(() => {
    document.documentElement.style.setProperty('--text-scale', String(textScale(textSize)))
  }, [textSize])

  // Handed to the truck (the sidebar's truck button, or the W shortcut): CarPlay shrinks into WestPlay's
  // button and the truck's own screen shows; the button grows it back.
  const truckScreen = useCabin((state) => state.truckScreen)
  const button = returnButtonCentre(profile, insets.left)
  // A video full screen: the rest of the CarPlay area steps aside for it (apps/youtube/youtube.css). A video
  // in small (YouTube's Now playing): the CarPlay area is cut open where it shows.
  const watching = useYouTube((state) => state.open)
  const holdSidebar = useYouTube((state) => state.holdSidebar)
  // YouTube Music's music video in its Now Playing, the same way.
  const youtubeHole = useYouTube((state) => state.hole)
  const musicHole = useYouTubeMusic((state) => state.hole)
  const hole = youtubeHole ?? musicHole
  // Over a video full screen, pointing at the left edge brings the sidebar back (shell/sidebarPeek.ts).
  const carplay = useRef<HTMLDivElement>(null)
  const peek = useSidebarPeek(watching, carplay)
  useEffect(() => {
    // Brought back, it goes again as soon as it's left, without the wait it has as YouTube opens.
    if (peek) useYouTube.setState({ holdSidebar: false })
  }, [peek])

  return (
    <main className={`shell appearance-${appearance}`} style={style}>
      {strip && !isInHost() && <GameStripPreview name={strip.name} />}
      <div
        ref={carplay}
        className={`carplay${truckScreen ? ' handed-over' : ''}${watching ? ' watching' : ''}${watching && holdSidebar ? ' hold-sidebar' : ''}${peek ? ' peek-sidebar' : ''}`}
        style={{
          transformOrigin: `${button.x - insets.left}px ${button.y}px`,
          ...(hole && !watching && hasVideoUnder() ? holeMask(hole) : {}),
        }}
        aria-hidden={truckScreen}
      >
        <Wallpaper src={wallpaperSrc} />
        <div className={entering ? 'carplay-content entering' : 'carplay-content'}>
          <Sidebar />
          {phoneSplit ? (
            <>
              <div className="phone-split-main">{children}</div>
              {phone?.beside}
            </>
          ) : (
            children
          )}
          <Player />
          <Keyboard />
          <VolumeHud />
        </div>
      </div>
      {/* What's drawn over the video where it shows through the CarPlay area (apps/youtube/VideoHole.tsx). */}
      <div id="video-over" className={truckScreen ? 'video-over handed-over' : 'video-over'} />
      <button
        type="button"
        className={truckScreen ? 'return-button shown' : 'return-button'}
        style={{ left: button.x - RETURN_BUTTON_SIZE / 2, top: button.y - RETURN_BUTTON_SIZE / 2 }}
        aria-label="WestPlay"
        aria-hidden={!truckScreen}
        tabIndex={truckScreen ? 0 : -1}
        onClick={() => useCabin.getState().setTruckScreen(false)}
      >
        <img src="/westplay.png" alt="" />
      </button>
      {overlay}
      {/* Over everything, the cursor included: the image's outermost pixels, which the game repeats into
          the screen's rim (screen.ts `rim`). Nothing moves for it: it covers only those. */}
      {profile.rim && <div className="screen-rim" style={{ color: profile.rim }} aria-hidden />}
    </main>
  )
}

/**
 * The player's fit of this screen (state/screenFit.ts) on the page's root: everything, menus and the keyboard
 * included, inside its margins and in its proportion, and black round it (the game's own screen would show there),
 * but while something under the page shows (`under`): the truck's own screen, handed to it, or YouTube's picture,
 * which the black, painted over all of what's under the page, hid (owner 2026-11-03: the video black on a fitted
 * screen, its controls over it). A transform on the root: what's clicked is still what's under the cursor.
 */
function useFittedRoot(
  { top, bottom, left, right, stretch }: ScreenFit,
  under: boolean,
  halved: 'mapLeft' | 'mapRight' | null,
) {
  useLayoutEffect(() => {
    const root = document.getElementById('root')
    if (!root) return
    const page = document.documentElement.style
    // The split screen: the page in its half, the rest of the screen see-through, the game's map there.
    if (halved) {
      const at = halved === 'mapLeft' ? '50vw' : '0'
      Object.assign(root.style, {
        position: 'fixed',
        left: at,
        top: '0',
        width: '50vw',
        height: '100vh',
        transform: 'none',
      })
      page.removeProperty('background')
      return
    }
    const style = fitStyle({ top, bottom, left, right, stretch, size: 1 })
    if (!style) {
      root.removeAttribute('style')
      page.removeProperty('background')
      return
    }
    Object.assign(root.style, { position: 'fixed', transformOrigin: '0 0', ...style })
    if (under) page.removeProperty('background')
    else page.setProperty('background', '#000')
  }, [top, bottom, left, right, stretch, under, halved])
}

/** A mask showing everything but a rounded rectangle, through which what's under the page shows. */
function holeMask({ x, y, width, height, radius }: Hole): CSSProperties {
  const shape = `<svg xmlns='http://www.w3.org/2000/svg' width='${width}' height='${height}'><rect width='${width}' height='${height}' rx='${radius}'/></svg>`
  return {
    maskImage: `linear-gradient(#000 0 0), url("data:image/svg+xml,${encodeURIComponent(shape)}")`,
    maskPosition: `0 0, ${x}px ${y}px`,
    maskSize: `100% 100%, ${width}px ${height}px`,
    maskRepeat: 'no-repeat',
    maskComposite: 'exclude',
  }
}

/** Browser previews have no game screen behind the page: mark where the game would draw its strip. */
function GameStripPreview({ name }: { name: string }) {
  return (
    <div className="game-strip-preview" aria-hidden>
      <span>{name} strip (drawn by the game)</span>
    </div>
  )
}
