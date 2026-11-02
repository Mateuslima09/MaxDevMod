import { installPhoneSocket } from '../phone/socket'
import { reportHealth } from './diagnostics'

/**
 * On the player's own phone (its own QR code, crates/host remote.rs), the way to the PC, in place before the cab's
 * code looks for it: the page is served by the PC, not shown in WestPlay's WebView2. In the game it has
 * WebView2's own.
 */
export const HANDSET_REMOTE = !window.chrome?.webview && installPhoneSocket()

// On the player's phone, how the page fares, into the PC's log (diagnostics.ts).
if (HANDSET_REMOTE) reportHealth((report) => window.chrome?.webview?.postMessage(report as never))
