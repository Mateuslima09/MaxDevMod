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

  // The page in the phone's shape: TikTok's side menu out of the way, no scroll bars; and as the app is (owner
  // 2026-11-11, with pictures: "the video full, the heart, the comments, the save over it, see-through"): each video
  // the whole page edge to edge, no card nor glow, its buttons over it on the right with nothing behind them, who and
  // what over it at the bottom left, only the thin line of how far it's played. By TikTok's own `data-e2e` names,
  // which its layouts keep.
  const style = document.createElement('style')
  style.textContent = `
[class*="DivSideNavPlaceholderContainer"],[class*="DivSideNavContainer"]{display:none!important}
::-webkit-scrollbar{display:none}
html,body{background:#000!important}
#app,[class*="BaseBodyContainer"],main,[data-e2e="anchoring-player-column"],#column-list-container{width:100vw!important;max-width:none!important;margin:0!important;padding:0!important;left:0!important}
#column-list-container{height:100vh!important}
[data-e2e="recommend-list-item-container"]{width:100vw!important;height:100vh!important;max-width:none!important;margin:0!important;padding:0!important;border:0!important;box-sizing:border-box!important}
[data-e2e="recommend-list-item-container"]>div:not([data-e2e="anchoring-ambient"]){position:relative!important;width:100vw!important;height:100vh!important;max-width:none!important;margin:0!important;padding:0!important;inset:0!important;top:0!important;gap:0!important}
[data-e2e="anchoring-ambient"],[data-e2e="anchoring-scroll-controls"]{display:none!important}
section[data-e2e="feed-video"]{position:relative!important;width:100vw!important;height:100vh!important;max-width:none!important;max-height:none!important;margin:0!important;border-radius:0!important;inset:0!important;top:0!important;flex:none!important}
section[data-e2e="feed-video"] *:has(video){border-radius:0!important}
section[data-e2e="feed-video"] video{position:absolute!important;inset:0!important;width:100%!important;height:100%!important;max-width:none!important;max-height:none!important;object-fit:cover!important}
html,body,main,#column-list-container,[data-e2e="anchoring-player-column"],[data-e2e="recommend-list-item-container"]{overflow-x:hidden!important}
[data-e2e="anchoring-engagement-vertical"]{position:absolute!important;right:6px!important;left:auto!important;top:auto!important;bottom:92px!important;z-index:20!important;gap:6px!important}
[data-e2e="recommend-list-item-container"] [class*="DivContentFlexLayout"]>:not(section[data-e2e="feed-video"]),[class*="SectionActionBarContainer"],[class*="DivActionBarWrapper"]{position:absolute!important;right:6px!important;left:auto!important;top:auto!important;bottom:92px!important;z-index:20!important;width:auto!important;height:auto!important;margin:0!important;padding:0!important;background:none!important;flex:none!important}
[data-e2e="recommend-list-item-container"] button:has([data-e2e$="-icon"]),[data-e2e="recommend-list-item-container"] [data-e2e="like-icon"],[data-e2e="recommend-list-item-container"] [data-e2e="comment-icon"],[data-e2e="recommend-list-item-container"] [data-e2e="favorite-icon"],[data-e2e="recommend-list-item-container"] [data-e2e="share-icon"]{background-color:transparent!important;box-shadow:none!important}
[data-e2e="recommend-list-item-container"] [data-e2e$="-icon"] svg{filter:drop-shadow(0 1px 2px rgb(0 0 0 / .6))}
[data-e2e="recommend-list-item-container"] [data-e2e$="-count"]{color:#fff!important;text-shadow:0 1px 2px rgb(0 0 0 / .8)}
[data-e2e="anchoring-engagement-vertical"] button,[data-e2e="anchoring-engagement-vertical"] [data-e2e$="-icon"],[data-e2e="anchoring-engagement-vertical"] [data-e2e$="-icon"] *{background-color:transparent!important;box-shadow:none!important}
[data-e2e="anchoring-engagement-vertical"] svg{filter:drop-shadow(0 1px 2px rgb(0 0 0 / .6))}
[data-e2e="anchoring-engagement-vertical"] [data-e2e$="-icon"] svg{width:30px!important;height:30px!important}
[data-e2e="anchoring-engagement-vertical"] strong{text-shadow:0 1px 2px rgb(0 0 0 / .8)}
[data-e2e="anchoring-description-layer"]{position:absolute!important;left:10px!important;right:64px!important;bottom:40px!important;top:auto!important;width:auto!important;z-index:20!important;text-shadow:0 1px 2px rgb(0 0 0 / .8)}
[data-e2e="anchoring-playback-chrome"]{position:absolute!important;left:0!important;right:0!important;bottom:0!important;top:auto!important;width:100%!important;height:auto!important;padding:0!important;background:none!important;z-index:21!important}
[data-e2e="anchoring-playback-chrome"]>*:not(:has([data-e2e="anchoring-progress-bar"])):not([data-e2e="anchoring-progress-bar"]){display:none!important}
[class*="DivBodyContainer"]:has([class*="DivLiveContentContainer"]),[class*="DivLiveContentContainer"]{width:100vw!important;min-width:0!important;max-width:none!important;margin-left:0!important;padding-left:0!important;left:0!important}
[data-westplay-hidden]{display:none!important}
`
  const addStyle = () => {
    const parent = document.head || document.documentElement
    if (parent && style.parentNode !== parent) parent.appendChild(style)
  }
  addStyle()
  document.addEventListener('DOMContentLoaded', addStyle)

  // The LIVE tab (owner 2026-11-12, with a picture): its own side column (the logo, back, the LIVEs followed,
  // "LIVE creators") is not the side menu the style above hides, and it squeezed the LIVEs to the right and cut
  // them. Found by where it is, as its names aren't known: what stands left of the LIVEs, as tall as most of the
  // page and not as wide as it, is put away, and the LIVEs take the whole width.
  const moved = new Set()
  const fitLive = () => {
    const onLive = location.pathname === '/live' || location.pathname.startsWith('/live/')
    if (!onLive) {
      for (const element of moved) {
        element.removeAttribute('data-westplay-hidden')
        element.style.removeProperty('margin-left')
        element.style.removeProperty('padding-left')
      }
      moved.clear()
      return
    }
    const content =
      document.querySelector('[class*="DivLiveContentContainer"]') ||
      document.querySelector('[class*="DivFeedTabContainer"]')
    if (!content) return
    // Beside the LIVEs it ends where they start; pinned over them, it just sits on the left edge.
    const column = (element, pinned) => {
      if (!(element instanceof HTMLElement) || element.contains(content) || element.hasAttribute('data-westplay-hidden'))
        return false
      const box = element.getBoundingClientRect()
      return (
        box.width > 0 &&
        box.width < innerWidth * 0.8 &&
        box.left < 4 &&
        box.height > innerHeight * 0.6 &&
        (pinned || box.right <= content.getBoundingClientRect().left + 4)
      )
    }
    const hide = (element) => {
      element.setAttribute('data-westplay-hidden', '')
      moved.add(element)
    }
    // Beside the LIVEs, at any level up to the page.
    for (let at = content; at && at !== document.body; at = at.parentElement) {
      const parent = at.parentElement
      if (!parent) break
      for (const sibling of parent.children) if (sibling !== at && column(sibling)) hide(sibling)
    }
    // Or pinned over the left edge of the page.
    for (const element of document.querySelectorAll('nav, aside, [class*="Nav"], [class*="Side"]')) {
      if (column(element, true) && ['fixed', 'sticky'].includes(getComputedStyle(element).position)) hide(element)
    }
    // And the room it left: the LIVEs from the left edge.
    for (let at = content; at && at !== document.documentElement; at = at.parentElement) {
      const look = getComputedStyle(at)
      if (parseFloat(look.marginLeft) > 0) {
        at.style.setProperty('margin-left', '0', 'important')
        moved.add(at)
      }
      if (parseFloat(look.paddingLeft) > 0 && at.getBoundingClientRect().left <= 0) {
        at.style.setProperty('padding-left', '0', 'important')
        moved.add(at)
      }
    }
  }
  setInterval(fitLive, 500)

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

  // The player's own controls (owner 2026-11-21: "the volume, the speed, the auto scroll aren't there any more"):
  // TikTok's are in the playback bar the style above puts away for the phone's shape, so the page has its own, at
  // the top right over the video: the volume (and a slider), the speed, and going on to the next video by itself.
  // Kept in the page's storage; the phone's side keys step the volume, shown on the left as iOS shows it.
  const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2]
  const player = { volume: 1, speed: 1, auto: false }
  try {
    const kept = JSON.parse(localStorage.getItem('westplay-player') || '{}')
    if (typeof kept.volume === 'number' && kept.volume >= 0 && kept.volume <= 1) player.volume = kept.volume
    if (SPEEDS.includes(kept.speed)) player.speed = kept.speed
    if (typeof kept.auto === 'boolean') player.auto = kept.auto
  } catch {
    // None kept, or unreadable: the defaults.
  }
  const keep = () => {
    try {
      localStorage.setItem('westplay-player', JSON.stringify(player))
    } catch {
      // Not kept: this visit's only.
    }
  }
  const onLivePage = () => location.pathname === '/live' || location.pathname.startsWith('/live/')
  const fitted = new WeakSet()
  // Each video as chosen: a new one at once, the others whenever TikTok set them otherwise.
  const apply = (all) => {
    for (const video of videos()) {
      const fresh = !fitted.has(video)
      if (fresh || all) {
        fitted.add(video)
        video.muted = player.volume === 0
      }
      if (Math.abs(video.volume - player.volume) > 0.005) video.volume = player.volume
      const live = onLivePage() || video.duration === Infinity
      const rate = live ? 1 : player.speed
      if (video.playbackRate !== rate) video.playbackRate = rate
    }
  }
  setInterval(() => {
    apply(false)
    drawControls()
  }, 400)
  document.addEventListener('loadedmetadata', () => apply(false), true)

  // The video most in view.
  const current = () => {
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
    return best
  }
  // On to the next video: the feed's next one scrolled to, else TikTok's own key for it.
  let movedAt = 0
  const next = () => {
    if (Date.now() - movedAt < 1500) return
    movedAt = Date.now()
    const video = current()
    const item = video && video.closest('[data-e2e="recommend-list-item-container"], article')
    let after = item && item.nextElementSibling
    while (after && !(after instanceof HTMLElement && after.getBoundingClientRect().height > 50)) after = after.nextElementSibling
    if (after) {
      after.scrollIntoView({ behavior: 'smooth', block: 'start' })
      return
    }
    for (const type of ['keydown', 'keyup'])
      document.dispatchEvent(
        new KeyboardEvent(type, { key: 'ArrowDown', code: 'ArrowDown', keyCode: 40, which: 40, bubbles: true }),
      )
  }
  // TikTok's videos go round: the one in view coming back to its start (or ending) is it played through.
  const lastTime = new WeakMap()
  document.addEventListener(
    'timeupdate',
    (event) => {
      const video = event.target
      if (!(video instanceof HTMLVideoElement)) return
      const before = lastTime.get(video) ?? 0
      lastTime.set(video, video.currentTime)
      if (!player.auto || !sounding || onLivePage() || !isFinite(video.duration) || video !== current()) return
      if (before > video.duration - 1 && video.currentTime < 0.6) next()
    },
    true,
  )
  document.addEventListener(
    'ended',
    (event) => {
      if (player.auto && sounding && !onLivePage() && event.target === current()) next()
    },
    true,
  )

  const SPEAKER = '<path d="M4 9h4l5-4v14l-5-4H4Z" fill="currentColor"/>'
  const WAVES = '<path d="M16 8.5a5 5 0 0 1 0 7M18.5 6a8.5 8.5 0 0 1 0 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>'
  const MUTED = '<path d="m16 9 6 6m0-6-6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>'
  const AUTO =
    '<path d="M12 4v12m-5-5 5 5 5-5" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/><path d="M6 20h12" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/>'
  const controlsStyle = document.createElement('style')
  controlsStyle.textContent = `
#westplay-player{position:fixed;top:10px;right:8px;z-index:2147483000;display:flex;align-items:center;gap:6px;font:600 12px/1 -apple-system,system-ui,sans-serif;color:#fff}
#westplay-player[hidden]{display:none}
#westplay-player button{all:unset;box-sizing:border-box;height:32px;min-width:32px;padding:0 8px;border-radius:16px;display:flex;align-items:center;justify-content:center;gap:4px;background:rgb(0 0 0 / .42);backdrop-filter:blur(10px);cursor:pointer;color:#fff;text-shadow:0 1px 2px rgb(0 0 0 / .6)}
#westplay-player button:active{background:rgb(255 255 255 / .25)}
#westplay-player button.on{background:#fe2c55}
#westplay-player svg{width:18px;height:18px;flex:none;filter:drop-shadow(0 1px 2px rgb(0 0 0 / .6))}
#westplay-player .slider{display:none;height:32px;padding:0 10px;border-radius:16px;align-items:center;background:rgb(0 0 0 / .42);backdrop-filter:blur(10px)}
#westplay-player .slider.open{display:flex}
#westplay-player input{width:96px;accent-color:#fff;cursor:pointer;margin:0}
#westplay-volume{position:fixed;left:8px;top:90px;width:28px;height:120px;border-radius:14px;overflow:hidden;background:rgb(60 60 66 / .75);backdrop-filter:blur(16px);z-index:2147483000;display:flex;align-items:flex-end;justify-content:center;color:#555;opacity:0;transition:opacity .2s;pointer-events:none}
#westplay-volume.shown{opacity:1}
#westplay-volume .fill{position:absolute;left:0;right:0;bottom:0;background:rgb(255 255 255 / .92);transition:height .12s}
#westplay-volume svg{position:relative;width:14px;height:14px;margin-bottom:8px}
`
  const bar = document.createElement('div')
  bar.id = 'westplay-player'
  bar.hidden = true
  bar.innerHTML =
    '<div class="slider"><input type="range" min="0" max="100" aria-label="Volume"></div>' +
    '<button type="button" data-do="volume" aria-label="Volume"><svg viewBox="0 0 24 24"></svg></button>' +
    '<button type="button" data-do="speed" aria-label="Velocidade"></button>' +
    `<button type="button" data-do="auto" aria-label="Rolagem automática"><svg viewBox="0 0 24 24">${AUTO}</svg>Auto</button>`
  const pill = document.createElement('div')
  pill.id = 'westplay-volume'
  pill.innerHTML = '<div class="fill"></div><svg viewBox="0 0 24 24"></svg>'
  const slider = bar.querySelector('.slider')
  const range = bar.querySelector('input')
  let sliderClosing = 0
  const openSlider = () => {
    slider.classList.add('open')
    clearTimeout(sliderClosing)
    sliderClosing = setTimeout(() => slider.classList.remove('open'), 4000)
  }
  let drawn = ''
  function drawControls() {
    const parent = document.body
    if (!parent) return
    if (controlsStyle.parentNode !== (document.head || parent)) (document.head || parent).appendChild(controlsStyle)
    if (bar.parentNode !== parent) parent.appendChild(bar)
    if (pill.parentNode !== parent) parent.appendChild(pill)
    const hasVideo = videos().some((video) => video.getBoundingClientRect().height > 100)
    bar.hidden = !hasVideo
    const live = onLivePage()
    const now = JSON.stringify([player, live])
    if (now === drawn) return
    drawn = now
    const icon = SPEAKER + (player.volume === 0 ? MUTED : WAVES)
    bar.querySelector('[data-do="volume"] svg').innerHTML = icon
    pill.querySelector('svg').innerHTML = icon
    pill.querySelector('.fill').style.height = `${Math.round(player.volume * 100)}%`
    if (document.activeElement !== range) range.value = String(Math.round(player.volume * 100))
    const speed = bar.querySelector('[data-do="speed"]')
    speed.textContent = `${player.speed}x`
    speed.style.display = live ? 'none' : ''
    const auto = bar.querySelector('[data-do="auto"]')
    auto.classList.toggle('on', player.auto)
    auto.style.display = live ? 'none' : ''
  }
  let lastVolume = player.volume || 1
  const setVolume = (volume) => {
    player.volume = Math.min(1, Math.max(0, Math.round(volume * 100) / 100))
    if (player.volume > 0) lastVolume = player.volume
    keep()
    apply(true)
    drawControls()
  }
  bar.addEventListener('click', (event) => {
    const button = event.target instanceof Element && event.target.closest('button')
    if (!button) return
    event.preventDefault()
    event.stopPropagation()
    const what = button.dataset.do
    if (what === 'volume') {
      if (slider.classList.contains('open')) setVolume(player.volume === 0 ? lastVolume : 0)
      openSlider()
    } else if (what === 'speed') {
      player.speed = SPEEDS[(SPEEDS.indexOf(player.speed) + 1) % SPEEDS.length]
      keep()
      apply(true)
      drawControls()
    } else if (what === 'auto') {
      player.auto = !player.auto
      keep()
      drawControls()
    }
  })
  range.addEventListener('input', () => {
    setVolume(Number(range.value) / 100)
    openSlider()
  })
  for (const type of ['pointerdown', 'mousedown', 'wheel'])
    bar.addEventListener(type, (event) => event.stopPropagation())
  let pillHiding = 0
  // A step of the phone's side keys (5%, as the cab's keybinds), shown a moment by the keys.
  const stepVolume = (step) => {
    const steps = 20
    const at = Math.round(player.volume * steps)
    setVolume(Math.min(steps, Math.max(0, at + (step > 0 ? 1 : -1))) / steps)
    pill.classList.add('shown')
    clearTimeout(pillHiding)
    pillHiding = setTimeout(() => pill.classList.remove('shown'), 1400)
  }
  document.addEventListener('DOMContentLoaded', drawControls)

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
    else if (message.action === 'volume' && (message.step === 1 || message.step === -1)) stepVolume(message.step)
  })

  post({ ready: true })
})()
