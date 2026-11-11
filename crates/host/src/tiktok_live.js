// The player's own TikTok LIVE, watched hidden (tiktok.rs) for the phone's notifications: who followed, who sent
// a gift, who shared, who subscribed. The page is TikTok's own LIVE page, as a viewer's browser opens it; this
// script reads the room's messages as they reach the page (TikTok's "webcast" messages, protobuf) and says only
// those four kinds on: a name, and a gift's name and count. Its picture and sound are held still: nothing of it
// shows or plays.
(() => {
  if (window.top !== window || location.hostname !== 'www.tiktok.com') return
  if (window.__westplayTikTokLive) return
  window.__westplayTikTokLive = true

  const post = (message) => {
    try {
      window.chrome.webview.postMessage(message)
    } catch {
      // Not in WestPlay's WebView: nothing to tell.
    }
  }
  const onLivePage = () => /^\/@[^/]+\/live/.test(location.pathname)

  // The LIVE's picture and sound aren't for anyone: held paused.
  document.addEventListener(
    'play',
    (event) => {
      if (event.target instanceof HTMLMediaElement) {
        event.target.muted = true
        event.target.pause()
      }
    },
    true,
  )

  // A protobuf message's fields: numbers (`value`) and length-delimited ones (`bytes`), `null` if it isn't one.
  const read = (bytes) => {
    const fields = []
    let at = 0
    const varint = () => {
      let result = 0
      let scale = 1
      for (let count = 0; count < 10; count++) {
        if (at >= bytes.length) throw new Error('short')
        const byte = bytes[at++]
        result += (byte & 0x7f) * scale
        scale *= 128
        if (!(byte & 0x80)) return result
      }
      throw new Error('long')
    }
    try {
      while (at < bytes.length) {
        const tag = varint()
        const no = Math.floor(tag / 8)
        const type = tag % 8
        if (no === 0) return null
        if (type === 0) fields.push({ no, value: varint() })
        else if (type === 2) {
          const length = varint()
          if (at + length > bytes.length) return null
          fields.push({ no, bytes: bytes.subarray(at, at + length) })
          at += length
        } else if (type === 1) at += 8
        else if (type === 5) at += 4
        else return null
      }
    } catch {
      return null
    }
    return fields
  }
  const bytesOf = (fields, no) => {
    const field = fields && fields.find((each) => each.no === no && each.bytes)
    return field ? field.bytes : null
  }
  const numberOf = (fields, no) => {
    const field = fields && fields.find((each) => each.no === no && each.value !== undefined)
    return field ? field.value : null
  }
  const decoder = new TextDecoder('utf-8', { fatal: true })
  const text = (bytes) => {
    if (!bytes) return null
    try {
      return decoder.decode(bytes)
    } catch {
      return null
    }
  }
  const textOf = (fields, no) => text(bytesOf(fields, no))
  // A name fit to show: a line, not long, nothing unprintable.
  const clean = (value) => {
    if (typeof value !== 'string') return null
    // eslint-disable-next-line no-control-regex
    const line = value.replace(/[\u0000-\u001f\u007f]/g, ' ').trim()
    return line && line.length <= 60 ? line : null
  }
  // Every text in a message, a few levels down (its display keys: "…follow…", "…share…").
  const texts = (bytes, depth) => {
    const fields = bytes && read(bytes)
    if (!fields) return []
    const found = []
    for (const field of fields) {
      if (!field.bytes) continue
      const inner = depth > 0 ? texts(field.bytes, depth - 1) : []
      if (inner.length) found.push(...inner)
      else {
        const said = text(field.bytes)
        if (said) found.push(said)
      }
    }
    return found
  }
  // A user's name as shown: their nickname, else their @.
  const nameOf = (bytes) => {
    const user = bytes && read(bytes)
    return user ? clean(textOf(user, 3)) || clean(textOf(user, 38)) : null
  }

  // Said once: the same person doing the same again within a while isn't news.
  const lately = new Map()
  const emit = (event) => {
    if (!onLivePage()) return
    const key = `${event.kind}|${event.user}|${event.detail || ''}`
    const now = Date.now()
    if (event.kind !== 'gift' && now - (lately.get(key) || 0) < 15000) return
    lately.set(key, now)
    if (lately.size > 300) lately.clear()
    post({ event })
  }

  // Gifts sent in a streak come as many messages: told once, as the streak ends (or stops coming).
  const streaks = new Map()
  const gift = (user, name, count, key, streak, ended) => {
    const pending = streaks.get(key)
    if (pending) clearTimeout(pending.timer)
    const tell = () => {
      streaks.delete(key)
      emit({ kind: 'gift', user, detail: name, count })
    }
    if (!streak || ended) tell()
    else streaks.set(key, { timer: setTimeout(tell, 3500) })
  }

  const handle = (method, payload) => {
    const message = read(payload)
    if (!message) return
    if (method === 'WebcastSocialMessage') {
      const user = nameOf(bytesOf(message, 2))
      const keys = texts(bytesOf(message, 1), 3).join(' ').toLowerCase()
      const action = numberOf(message, 4)
      const kind = keys.includes('follow')
        ? 'follow'
        : keys.includes('share')
          ? 'share'
          : action === 1
            ? 'follow'
            : action === 3
              ? 'share'
              : null
      if (kind && user) emit({ kind, user })
    } else if (method === 'WebcastGiftMessage') {
      const user = nameOf(bytesOf(message, 7))
      if (!user) return
      const details = read(bytesOf(message, 15) || new Uint8Array())
      const name = details ? clean(textOf(details, 16)) : null
      const streak = details ? numberOf(details, 11) === 1 : false
      const count = Math.max(1, Math.min(9999, numberOf(message, 5) || 1))
      const id = numberOf(message, 2) || 0
      gift(user, name, count, `${user}|${id}`, streak, numberOf(message, 9) === 1)
    } else if (method === 'WebcastSubNotifyMessage') {
      const user = nameOf(bytesOf(message, 2))
      if (user) emit({ kind: 'subscribe', user })
    }
  }

  // The room is on: said every so often while its messages come.
  let told = 0
  const alive = () => {
    const now = Date.now()
    if (onLivePage() && now - told > 15000) {
      told = now
      post({ live: true })
    }
  }

  const unzip = async (bytes) => {
    if (bytes[0] !== 0x1f || bytes[1] !== 0x8b) return bytes
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))
    return new Uint8Array(await new Response(stream).arrayBuffer())
  }

  // What came from the room: the socket's frame (its payload the response, zipped), or the response itself.
  const frame = async (buffer) => {
    try {
      let fields = read(new Uint8Array(buffer))
      if (!fields) return
      const payload = bytesOf(fields, 8)
      if (payload && textOf(fields, 7) !== null) {
        if (textOf(fields, 7) !== 'msg') return
        fields = read(await unzip(payload))
        if (!fields) return
      }
      let any = false
      for (const field of fields) {
        if (field.no !== 1 || !field.bytes) continue
        const message = read(field.bytes)
        const method = message && textOf(message, 1)
        const body = message && bytesOf(message, 2)
        if (!method || !method.startsWith('Webcast') || !body) continue
        any = true
        try {
          handle(method, body)
        } catch {
          // A message TikTok changed: the others still read.
        }
      }
      if (any) alive()
    } catch {
      // Not a room's message.
    }
  }

  const NativeSocket = window.WebSocket
  window.WebSocket = new Proxy(NativeSocket, {
    construct(target, args) {
      const socket = new target(...args)
      try {
        if (/webcast/i.test(String(args[0]))) {
          socket.addEventListener('message', (event) => {
            const data = event.data
            if (data instanceof ArrayBuffer) frame(data)
            else if (data instanceof Blob) data.arrayBuffer().then(frame)
          })
        }
      } catch {
        // The socket is the page's all the same.
      }
      return socket
    },
  })

  const nativeFetch = window.fetch
  window.fetch = function (...args) {
    const answer = nativeFetch.apply(this, args)
    try {
      const url = String((args[0] && args[0].url) || args[0])
      if (url.includes('/webcast/im/fetch')) {
        answer.then((response) => response.clone().arrayBuffer().then(frame)).catch(() => {})
      }
    } catch {
      // The request is the page's all the same.
    }
    return answer
  }

  post({ ready: true })
})()
