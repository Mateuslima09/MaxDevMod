// TikTok's own site in the phone in the game (tiktok.rs): the page as TikTok serves it, used with the mouse as
// in a browser. This script only fits it to the phone (its side menu hidden: the phone's own tabs go to those
// pages), keeps it quiet while the phone is put away, says whether it's signed in (and as whom: the player's own
// LIVE is watched for the phone's notifications), and types what the player's keyboard typed into its fields.
// Nothing of the account but its name leaves the page.
(() => {
  if (window.top !== window || location.hostname !== 'www.tiktok.com') return
  if (window.__westplayTikTok) return
  window.__westplayTikTok = true

  const post = (message) => {
    try {
      window.chrome.webview.postMessage(message)
    } catch {
      // Not in WestPlay's WebView: nothing to tell.
    }
  }

  // The page in the phone's shape: TikTok's side menu out of the way, no scroll bars.
  const style = document.createElement('style')
  style.textContent =
    '[class*="DivSideNavPlaceholderContainer"],[class*="DivSideNavContainer"]{display:none!important}' +
    '::-webkit-scrollbar{display:none}'
  const addStyle = () => {
    const parent = document.head || document.documentElement
    if (parent && style.parentNode !== parent) parent.appendChild(style)
  }
  addStyle()
  document.addEventListener('DOMContentLoaded', addStyle)

  // Quiet while the phone is put away (or another of its apps is open): every video paused, and held paused.
  let sounding = false
  const videos = () => Array.from(document.querySelectorAll('video'))
  const quiet = () => {
    for (const video of videos()) if (!video.paused) video.pause()
  }
  document.addEventListener(
    'play',
    (event) => {
      if (!sounding && event.target instanceof HTMLMediaElement) event.target.pause()
    },
    true,
  )
  // Back out: the video most in view plays on.
  const resume = () => {
    let best = null
    let most = 0
    for (const video of videos()) {
      const box = video.getBoundingClientRect()
      const seen = Math.max(0, Math.min(box.bottom, innerHeight) - Math.max(box.top, 0))
      if (seen > most) {
        most = seen
        best = video
      }
    }
    if (best && best.paused) best.play().catch(() => {})
  }

  // Whether it's signed in, and the account's name: TikTok's own answer to its own page.
  let said = ''
  const check = async () => {
    let state = { signedIn: false, user: null }
    try {
      const answer = await fetch('/passport/web/account/info/?aid=1459', { credentials: 'include' })
      const info = await answer.json()
      const name = info && info.message === 'success' && info.data && info.data.username
      if (typeof name === 'string' && name) state = { signedIn: true, user: name }
    } catch {
      // No answer: said again at the next check.
      return
    }
    const now = JSON.stringify(state)
    if (now !== said) {
      said = now
      post({ state })
    }
  }
  check()
  setInterval(check, 30000)

  // The field the player's keyboard types into: any of the page's (a search, a comment).
  const editable = (element) =>
    element instanceof HTMLElement &&
    (element.isContentEditable ||
      element instanceof HTMLTextAreaElement ||
      (element instanceof HTMLInputElement && !['checkbox', 'radio', 'button', 'submit'].includes(element.type)))
  let typing = false
  const sayTyping = () => {
    const now = editable(document.activeElement)
    if (now !== typing) {
      typing = now
      post({ typing })
    }
  }
  // Its focus events come only to a window that has the keyboard, which WestPlay's never has (the game's does):
  // looked at after each click too, and every so often.
  document.addEventListener('focusin', sayTyping, true)
  document.addEventListener('focusout', () => setTimeout(sayTyping, 0), true)
  document.addEventListener('click', () => setTimeout(sayTyping, 60), true)
  setInterval(sayTyping, 700)

  const key = (name, code) => {
    const target = document.activeElement || document.body
    for (const type of ['keydown', 'keypress', 'keyup']) {
      target.dispatchEvent(
        new KeyboardEvent(type, { key: name, code: name, keyCode: code, which: code, bubbles: true, cancelable: true }),
      )
    }
  }
  const typed = (events) => {
    if (!editable(document.activeElement)) return
    for (const event of events) {
      if (typeof event.text === 'string') document.execCommand('insertText', false, event.text)
      else if (event.key === 'backspace') document.execCommand('delete')
      else if (event.key === 'wordBackspace') {
        const selection = getSelection()
        if (selection && selection.modify) selection.modify('extend', 'backward', 'word')
        document.execCommand('delete')
      } else if (event.key === 'enter') key('Enter', 13)
      else if (event.key === 'escape') {
        key('Escape', 27)
        if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
      }
    }
  }

  window.chrome.webview.addEventListener('message', (event) => {
    const message = event.data
    if (!message || typeof message !== 'object') return
    if (message.type === 'typed' && Array.isArray(message.events)) typed(message.events)
    else if (message.action === 'sound') {
      sounding = message.on === true
      if (sounding) resume()
      else quiet()
    } else if (message.action === 'check') check()
    else if (message.action === 'back') history.back()
  })

  post({ ready: true })
})()
