import { useEffect, useState } from 'react'
import { Screens } from '../cabin/CabinApp'
import { installDragScroll } from '../cabin/shell/dragScroll'
import { Shell } from '../cabin/shell/Shell'
import { useCabin } from '../cabin/state/store'
import { SCREEN_WIDTH } from '../cabin/state/screen'

/**
 * The truck's panel on the phone held sideways (owner 2026-11-01: "as if I had the panel"): the cab's own CarPlay,
 * its Home, its apps and its Dashboard, laid out as wide as the cab's screen and scaled to the phone's. Its screens
 * are its own (opening an app here doesn't open it in the cab), what it does is the cab's: the music it plays, the
 * volume it sets (shown in the cab), the route it hands the game.
 */
export function PhonePanel() {
  const route = useCabin((state) => state.navigation.route)
  const size = usePanelSize()
  // Lists follow a finger as they follow the cab's cursor, with the same glide.
  useEffect(() => installDragScroll(), [])
  return (
    <div className="phone-panel" style={{ width: SCREEN_WIDTH, height: size.height, zoom: size.zoom }}>
      <Shell entering={false} overlay={null}>
        <Screens route={route} />
      </Shell>
    </div>
  )
}

/** The panel's height, at the cab screen's width, and how much it's scaled to fill the phone's screen. */
function usePanelSize(): { height: number; zoom: number } {
  const measure = () => {
    const zoom = window.innerWidth / SCREEN_WIDTH
    return { height: Math.round(window.innerHeight / zoom), zoom }
  }
  const [size, setSize] = useState(measure)
  useEffect(() => {
    const follow = () => setSize(measure())
    window.addEventListener('resize', follow)
    return () => window.removeEventListener('resize', follow)
  }, [])
  return size
}
