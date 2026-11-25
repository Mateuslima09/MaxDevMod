import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { postToHost, type AudioOutput, type YouTubeChannel, type YouTubePlaylist, type YouTubeVideo } from '../shared/host'
import { useCharacterSwitches } from '../cabin/state/store'
import { usePrefs } from './prefs'
import { SearchGlyph } from '../cabin/keyboard/glyphs'
import { useKeyboard } from '../cabin/keyboard/store'
import { TextField } from '../cabin/keyboard/TextField'
import { formatClock } from '../cabin/apps/youtube/time'
import {
  ask,
  comingUp,
  command,
  nextVideo,
  play,
  positionNow,
  rememberSearch,
  seekBy,
  setLanguage,
  setQuality,
  signIn,
  signOut,
  stopVideo,
  toggleMute,
  useYouTube,
} from '../cabin/apps/youtube/youtube'

const FIELD_ID = 'ipad-youtube-search'
const CONTROLS_HIDE_MS = 3500

type Tab = 'home' | 'subscriptions' | 'upNext' | 'history'

type BrowseView =
  | { kind: 'tabs' }
  | { kind: 'search'; query: string }
  | { kind: 'channel'; channel: Pick<YouTubeChannel, 'channel' | 'name'> & Partial<YouTubeChannel> }
  | { kind: 'playlist'; playlist: Pick<YouTubePlaylist, 'playlist' | 'title'> & Partial<YouTubePlaylist> }

export function IpadYouTubeApp({
  language = 'pt',
  remote = false,
  onHome,
}: {
  language?: 'en' | 'pt'
  remote?: boolean
  onHome: () => void
}) {
  const words = WORDS[language]
  const current = useYouTube((s) => s.current)
  const state = useYouTube((s) => s.state)
  const account = useYouTube((s) => s.account)
  const signingIn = useYouTube((s) => s.signingIn)
  const watched = useYouTube((s) => s.watched)
  const searches = useYouTube((s) => s.searches)
  const upNext = useYouTube((s) => s.upNext)
  const queue = useYouTube((s) => s.queue)
  const muted = useYouTube((s) => s.muted)
  const volume = useYouTube((s) => s.volume)
  const quality = useYouTube((s) => s.quality)
  const audioOutput = useCharacterSwitches((s) => s.audioOutput)
  const deviceKind = usePrefs().device
  const toggleAudioOutput = useCallback(() => {
    const next: AudioOutput = audioOutput === 'device' ? 'cabin' : 'device'
    useCharacterSwitches.setState({ audioOutput: next })
    postToHost({ type: 'audioOutput', which: next })
  }, [audioOutput])
  const deviceLabel = deviceKind === 'ipad' ? words.audioIpad : words.audioPhone
  const audioLabel = audioOutput === 'device' ? deviceLabel : words.audioCabin

  const [query, setQuery] = useState('')
  const [tab, setTab] = useState<Tab>('home')
  const [view, setView] = useState<BrowseView>({ kind: 'tabs' })
  const [activeChip, setActiveChip] = useState<string | null>(null)
  const [suggestions, setSuggestions] = useState<string[]>([])
  const [liveVideos, setLiveVideos] = useState<YouTubeVideo[]>([])
  const [liveLoading, setLiveLoading] = useState(false)
  const [videos, setVideos] = useState<YouTubeVideo[]>([])
  const [channels, setChannels] = useState<YouTubeChannel[]>([])
  const [playlists, setPlaylists] = useState<YouTubePlaylist[]>([])
  const [moreToken, setMoreToken] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [failed, setFailed] = useState(false)
  const [accountMenu, setAccountMenu] = useState(false)

  const typing = useKeyboard((s) => s.field?.id === FIELD_ID)
  const reqSeq = useRef(0)
  const liveSeq = useRef(0)

  // Start YouTube engine and sync language on mount; hide video when leaving the iPad YouTube app.
  useEffect(() => {
    setLanguage(language)
    return () => {
      if (!remote) command({ action: 'hide' })
    }
  }, [language, remote])

  // Fetch search suggestions + live video preview as the user types.
  useEffect(() => {
    const trimmed = query.trim()
    if (!typing || !trimmed) {
      setSuggestions([])
      setLiveVideos([])
      setLiveLoading(false)
      return
    }
    let cancelled = false
    const sugTimer = setTimeout(() => {
      ask({ action: 'suggest', query: trimmed }).then((ans) => {
        if (!cancelled) setSuggestions((ans.suggestions ?? []).slice(0, 5))
      })
    }, 180)
    const vidTimer = setTimeout(() => {
      const seq = ++liveSeq.current
      setLiveLoading(true)
      ask({ action: 'search', query: trimmed }).then((ans) => {
        if (cancelled || liveSeq.current !== seq) return
        setLiveLoading(false)
        if (!ans.failed) setLiveVideos((ans.videos ?? []).slice(0, 8))
      })
    }, 380)
    return () => {
      cancelled = true
      clearTimeout(sugTimer)
      clearTimeout(vidTimer)
    }
  }, [query, typing])

  // Load feed / search / channel / playlist whenever the active view or tab changes.
  const loadContent = useCallback(
    async (targetView: BrowseView, targetTab: Tab) => {
      if (targetView.kind === 'tabs' && (targetTab === 'history' || targetTab === 'upNext')) {
        setLoading(false)
        setFailed(false)
        return
      }
      if (targetView.kind === 'tabs' && !account?.signedIn) {
        setVideos([])
        setChannels([])
        setPlaylists([])
        setMoreToken(null)
        setLoading(false)
        setFailed(false)
        return
      }
      const seq = ++reqSeq.current
      setLoading(true)
      setFailed(false)
      setVideos([])
      setChannels([])
      setPlaylists([])
      setMoreToken(null)

      const question =
        targetView.kind === 'search'
          ? ({ action: 'search', query: targetView.query } as const)
          : targetView.kind === 'channel'
            ? ({ action: 'channel', channel: targetView.channel.channel } as const)
            : targetView.kind === 'playlist'
              ? ({ action: 'playlist', playlist: targetView.playlist.playlist } as const)
              : ({ action: 'feed', which: targetTab === 'subscriptions' ? 'subscriptions' : 'home' } as const)

      const ans = await ask(question)
      if (reqSeq.current !== seq) return
      setLoading(false)
      if (ans.failed) {
        setFailed(true)
        return
      }
      setVideos(ans.videos ?? [])
      setChannels(ans.channels ?? [])
      setPlaylists(ans.playlists ?? [])
      setMoreToken(ans.more ?? null)
    },
    [account?.signedIn],
  )

  useEffect(() => {
    void loadContent(view, tab)
  }, [view, tab, loadContent])

  const loadMore = async () => {
    if (!moreToken || loadingMore) return
    const seq = reqSeq.current
    setLoadingMore(true)
    const question =
      view.kind === 'search'
        ? ({ action: 'search', query: view.query, more: moreToken } as const)
        : view.kind === 'channel'
          ? ({ action: 'channel', channel: view.channel.channel, more: moreToken } as const)
          : view.kind === 'playlist'
            ? ({ action: 'playlist', playlist: view.playlist.playlist, more: moreToken } as const)
            : ({ action: 'feed', which: tab === 'subscriptions' ? 'subscriptions' : 'home', more: moreToken } as const)
    const ans = await ask(question)
    if (reqSeq.current !== seq) return
    setLoadingMore(false)
    if (ans.failed) return
    setVideos((prev) => {
      const seen = new Set(prev.map((v) => v.video))
      const added = (ans.videos ?? []).filter((v) => !seen.has(v.video))
      return [...prev, ...added]
    })
    setMoreToken(ans.more ?? null)
  }

  const submitSearch = (text: string) => {
    const trimmed = text.trim()
    if (!trimmed) return
    rememberSearch(trimmed)
    setQuery(trimmed)
    setActiveChip(null)
    useKeyboard.getState().detach(FIELD_ID)
    setView({ kind: 'search', query: trimmed })
  }

  const pickCategoryChip = (chip: CategoryChip | null) => {
    useKeyboard.getState().detach(FIELD_ID)
    if (!chip) {
      setActiveChip(null)
      setTab('home')
      setView({ kind: 'tabs' })
      return
    }
    setActiveChip(chip.id)
    setTab('home')
    setView({ kind: 'search', query: chip.query })
  }

  const pickVideo = (video: YouTubeVideo) => {
    useKeyboard.getState().detach(FIELD_ID)
    if (view.kind === 'playlist') {
      play(video, false, {
        playlist: view.playlist.playlist,
        title: view.playlist.title,
        videos,
        more: moreToken,
      })
    } else {
      play(video, false)
    }
  }

  const handleBack = () => {
    useKeyboard.getState().detach(FIELD_ID)
    if (view.kind !== 'tabs') {
      setActiveChip(null)
      setView({ kind: 'tabs' })
      return
    }
    onHome()
  }

  const coming = comingUp({ queue, upNext, current })
  const hasVideo = current !== null

  return (
    <div className="yt-ipad-app">
      {/* Top solid bar: Back, YouTube logo, Search field, Account */}
      <header className="yt-ipad-header">
        <button type="button" className="yt-ipad-back" onClick={handleBack}>
          <svg viewBox="0 0 12 20" aria-hidden>
            <path d="M10 2 2 10l8 8" />
          </svg>
          <span>{view.kind === 'tabs' ? words.home : words.back}</span>
        </button>

        <div className="yt-ipad-search-wrap">
          {remote ? (
            <input
              type="search"
              className="yt-ipad-search-input"
              placeholder={words.search}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') submitSearch(query)
              }}
            />
          ) : (
            <TextField
              id={FIELD_ID}
              value={query}
              onChange={setQuery}
              onSubmit={() => submitSearch(query)}
              returnKind="search"
              hint={words.search}
              label={words.search}
              icon={<SearchGlyph />}
            />
          )}
        </div>

        <button
          type="button"
          className={`yt-ipad-audio-btn${audioOutput === 'device' ? ' is-device' : ''}`}
          onClick={toggleAudioOutput}
          title={words.audioOutputTitle}
        >
          <span className="yt-ipad-audio-ico" aria-hidden>
            {audioOutput === 'device' ? (
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.1">
                <rect x="6" y="2.5" width="12" height="19" rx="2.4" />
                <path d="M10.5 18h3" strokeLinecap="round" />
              </svg>
            ) : (
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.1">
                <path d="M11 5 6 9H3v6h3l5 4V5Z" strokeLinejoin="round" />
                <path d="M15.5 8.5a5 5 0 0 1 0 7M18.5 6a8.5 8.5 0 0 1 0 12" strokeLinecap="round" />
              </svg>
            )}
          </span>
          <span>{audioLabel}</span>
        </button>

        <div className="yt-ipad-account-wrap">
          {account?.signedIn ? (
            <button
              type="button"
              className="yt-ipad-avatar-btn"
              onClick={() => setAccountMenu((v) => !v)}
              aria-label={words.account}
            >
              {account.avatar ? <img src={account.avatar} alt="" /> : <span>YT</span>}
            </button>
          ) : (
            <button
              type="button"
              className="yt-ipad-signin-btn"
              onClick={() => signIn()}
              disabled={signingIn}
            >
              {signingIn ? words.signingIn : words.signIn}
            </button>
          )}
          {accountMenu && account?.signedIn && (
            <div className="yt-ipad-account-menu">
              <button
                type="button"
                onClick={() => {
                  setAccountMenu(false)
                  signOut()
                }}
              >
                {words.signOut}
              </button>
            </div>
          )}
        </div>
      </header>

      {/* Pinned 16:9 Live Video Player (transparent hole with YouTube WebView underneath) */}
      {hasVideo && (
        <VideoStage
          video={current}
          state={state}
          muted={muted}
          volume={volume}
          quality={quality}
          audioOutput={audioOutput}
          audioLabel={audioLabel}
          onToggleAudioOutput={toggleAudioOutput}
          remote={remote}
          words={words}
          onNext={() => {
            const next = nextVideo()
            if (next) play(next, false)
          }}
          onStop={() => {
            stopVideo()
            if (!remote) command({ action: 'hide' })
          }}
        />
      )}

      {/* Solid dark scrollable body below the video */}
      <div className="yt-ipad-body">
        {typing ? (
          <div className="yt-ipad-suggestions">
            {query.trim() && (
              <button type="button" className="yt-ipad-sug-row primary" onClick={() => submitSearch(query)}>
                <span className="yt-ipad-sug-ico" aria-hidden>
                  <SearchGlyph />
                </span>
                <span>{query.trim()}</span>
              </button>
            )}
            {suggestions.map((s) => (
              <button key={s} type="button" className="yt-ipad-sug-row" onClick={() => submitSearch(s)}>
                <span className="yt-ipad-sug-ico" aria-hidden>
                  <SearchGlyph />
                </span>
                <span>{s}</span>
              </button>
            ))}
            {query.trim() && (liveLoading || liveVideos.length > 0) && (
              <div className="yt-ipad-live-results">
                <div className="yt-ipad-section-title">
                  {liveLoading ? words.loading : words.liveResults}
                </div>
                {liveVideos.length > 0 && (
                  <VideoGrid videos={liveVideos} activeId={current?.video} onPick={pickVideo} />
                )}
              </div>
            )}
            {!query.trim() && searches.length > 0 && (
              <>
                <div className="yt-ipad-section-title">{words.recentSearches}</div>
                {searches.map((s) => (
                  <button key={s} type="button" className="yt-ipad-sug-row" onClick={() => submitSearch(s)}>
                    <span className="yt-ipad-sug-hist">↺</span>
                    <span>{s}</span>
                  </button>
                ))}
              </>
            )}
          </div>
        ) : (
          <>
            {/* Navigation tabs or search/channel/playlist subheader */}
            {view.kind === 'tabs' || activeChip !== null ? (
              <div className="yt-ipad-nav-stack">
                <div className="yt-ipad-tabs">
                  <button
                    type="button"
                    className={tab === 'home' && activeChip === null ? 'active' : ''}
                    onClick={() => pickCategoryChip(null)}
                  >
                    {words.tabHome}
                  </button>
                  {account?.signedIn && (
                    <button
                      type="button"
                      className={tab === 'subscriptions' && activeChip === null ? 'active' : ''}
                      onClick={() => {
                        setActiveChip(null)
                        setView({ kind: 'tabs' })
                        setTab('subscriptions')
                      }}
                    >
                      {words.tabSubs}
                    </button>
                  )}
                  {hasVideo && (
                    <button
                      type="button"
                      className={tab === 'upNext' && activeChip === null ? 'active' : ''}
                      onClick={() => {
                        setActiveChip(null)
                        setView({ kind: 'tabs' })
                        setTab('upNext')
                      }}
                    >
                      {words.tabUpNext}
                    </button>
                  )}
                  <button
                    type="button"
                    className={tab === 'history' && activeChip === null ? 'active' : ''}
                    onClick={() => {
                      setActiveChip(null)
                      setView({ kind: 'tabs' })
                      setTab('history')
                    }}
                  >
                    {words.tabHistory}
                  </button>
                </div>

                {/* YouTube-style horizontal category shortcut chips bar */}
                {(tab === 'home' || activeChip !== null) && (
                  <div className="yt-ipad-catbar" role="tablist" aria-label="Categorias">
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activeChip === null}
                      className={`yt-ipad-cat${activeChip === null ? ' active' : ''}`}
                      onClick={() => pickCategoryChip(null)}
                    >
                      {words.chipAll}
                    </button>
                    {CATEGORY_CHIPS[language].map((chip) => (
                      <button
                        key={chip.id}
                        type="button"
                        role="tab"
                        aria-selected={activeChip === chip.id}
                        className={`yt-ipad-cat${activeChip === chip.id ? ' active' : ''}`}
                        onClick={() => pickCategoryChip(chip)}
                      >
                        {chip.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="yt-ipad-subhead">
                <span className="yt-ipad-subhead-title">
                  {view.kind === 'search'
                    ? `“${view.query}”`
                    : view.kind === 'channel'
                      ? view.channel.name
                      : view.playlist.title}
                </span>
                <button
                  type="button"
                  className="yt-ipad-subhead-clear"
                  onClick={() => {
                    setActiveChip(null)
                    setView({ kind: 'tabs' })
                  }}
                >
                  {words.tabHome}
                </button>
              </div>
            )}

            {/* Up Next tab */}
            {view.kind === 'tabs' && tab === 'upNext' && (
              <div className="yt-ipad-section">
                {coming.length === 0 ? (
                  <p className="yt-ipad-empty">{words.loading}</p>
                ) : (
                  <VideoGrid videos={coming} activeId={current?.video} onPick={pickVideo} />
                )}
              </div>
            )}

            {/* History tab */}
            {view.kind === 'tabs' && tab === 'history' && (
              <div className="yt-ipad-section">
                {watched.length === 0 ? (
                  <p className="yt-ipad-empty">{words.emptyHistory}</p>
                ) : (
                  <VideoGrid
                    videos={watched.map((w) => w.video)}
                    activeId={current?.video}
                    onPick={pickVideo}
                  />
                )}
              </div>
            )}

            {/* Signed-out Home view: Continue watching + quick search topics */}
            {view.kind === 'tabs' && tab === 'home' && !account?.signedIn && (
              <div className="yt-ipad-section">
                <div className="yt-ipad-welcome">
                  <div className="yt-ipad-welcome-copy">
                    <strong>YouTube</strong>
                    <span>{words.signedOutHint}</span>
                  </div>
                  <button type="button" className="yt-ipad-signin-btn" onClick={() => signIn()} disabled={signingIn}>
                    {signingIn ? words.signingIn : words.signIn}
                  </button>
                </div>

                <div className="yt-ipad-chips">
                  {QUICK_TOPICS[language].map((topic) => (
                    <button key={topic} type="button" className="yt-ipad-topic" onClick={() => submitSearch(topic)}>
                      {topic}
                    </button>
                  ))}
                </div>

                {watched.length > 0 && (
                  <>
                    <div className="yt-ipad-section-title">{words.continueWatching}</div>
                    <VideoGrid
                      videos={watched.map((w) => w.video)}
                      activeId={current?.video}
                      onPick={pickVideo}
                    />
                  </>
                )}
              </div>
            )}

            {/* Feed / Search / Channel / Playlist results */}
            {(view.kind !== 'tabs' || (account?.signedIn && (tab === 'home' || tab === 'subscriptions'))) && (
              <div className="yt-ipad-section">
                {loading && <p className="yt-ipad-empty">{words.loading}</p>}
                {!loading && failed && (
                  <div className="yt-ipad-empty-box">
                    <p className="yt-ipad-empty">{words.failed}</p>
                    <button
                      type="button"
                      className="yt-ipad-retry"
                      onClick={() => void loadContent(view, tab)}
                    >
                      {words.retry}
                    </button>
                  </div>
                )}

                {!loading && !failed && channels.length > 0 && (
                  <div className="yt-ipad-channels">
                    {channels.slice(0, 3).map((ch) => (
                      <button
                        key={ch.channel}
                        type="button"
                        className="yt-ipad-channel-pill"
                        onClick={() => setView({ kind: 'channel', channel: ch })}
                      >
                        {ch.avatar && <img src={ch.avatar} alt="" />}
                        <div className="yt-ipad-channel-meta">
                          <strong>{ch.name}</strong>
                          {ch.subscribers && <span>{ch.subscribers}</span>}
                        </div>
                      </button>
                    ))}
                  </div>
                )}

                {!loading && !failed && playlists.length > 0 && (
                  <div className="yt-ipad-channels">
                    {playlists.slice(0, 2).map((pl) => (
                      <button
                        key={pl.playlist}
                        type="button"
                        className="yt-ipad-channel-pill"
                        onClick={() => setView({ kind: 'playlist', playlist: pl })}
                      >
                        {pl.thumbnail && <img src={pl.thumbnail} alt="" className="is-pl" />}
                        <div className="yt-ipad-channel-meta">
                          <strong>{pl.title}</strong>
                          <span>{pl.count || pl.channel}</span>
                        </div>
                      </button>
                    ))}
                  </div>
                )}

                {!loading && !failed && videos.length > 0 && (
                  <VideoGrid videos={videos} activeId={current?.video} onPick={pickVideo} />
                )}

                {!loading && !failed && videos.length === 0 && channels.length === 0 && (
                  <p className="yt-ipad-empty">{words.noResults}</p>
                )}

                {!loading && moreToken && (
                  <button
                    type="button"
                    className="yt-ipad-more"
                    onClick={() => void loadMore()}
                    disabled={loadingMore}
                  >
                    {loadingMore ? words.loading : words.loadMore}
                  </button>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}

function VideoGrid({
  videos,
  activeId,
  onPick,
}: {
  videos: YouTubeVideo[]
  activeId?: string
  onPick: (video: YouTubeVideo) => void
}) {
  return (
    <div className="yt-ipad-grid">
      {videos.map((v) => {
        const active = activeId === v.video
        const meta = [v.channel, v.views, v.age].filter(Boolean).join(' · ')
        return (
          <button
            key={v.video}
            type="button"
            className={`yt-ipad-card${active ? ' is-playing' : ''}`}
            onClick={() => onPick(v)}
          >
            <div className="yt-ipad-thumb">
              {v.thumbnail && <img src={v.thumbnail} alt="" loading="lazy" />}
              {v.live ? (
                <span className="yt-ipad-badge is-live">LIVE</span>
              ) : v.duration ? (
                <span className="yt-ipad-badge">{v.duration}</span>
              ) : null}
            </div>
            <div className="yt-ipad-card-info">
              <span className="yt-ipad-card-title">{v.title}</span>
              <span className="yt-ipad-card-meta">{meta}</span>
            </div>
          </button>
        )
      })}
    </div>
  )
}

function VideoStage({
  video,
  state,
  muted,
  volume,
  quality,
  audioOutput,
  audioLabel,
  onToggleAudioOutput,
  remote,
  words,
  onNext,
  onStop,
}: {
  video: YouTubeVideo
  state: ReturnType<typeof useYouTube.getState>['state']
  muted: boolean
  volume: number
  quality: string
  audioOutput: AudioOutput
  audioLabel: string
  onToggleAudioOutput: () => void
  remote: boolean
  words: Words
  onNext: () => void
  onStop: () => void
}) {
  const holeRef = useRef<HTMLDivElement>(null)
  const [showControls, setShowControls] = useState(true)
  const [tick, setTick] = useState(0)
  const [qualityOpen, setQualityOpen] = useState(false)
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  const mine = state !== null && state.video === video.video
  const playing = mine && state.state === 1
  const buffering = !mine || state.state === -1 || state.state === 3

  // Refresh time progress smoothly while playing.
  useEffect(() => {
    if (!playing) return
    const id = setInterval(() => setTick((t) => t + 1), 500)
    return () => clearInterval(id)
  }, [playing])

  // Auto-hide controls after a few seconds while playing.
  const bumpControls = useCallback(() => {
    setShowControls(true)
    clearTimeout(hideTimer.current)
    hideTimer.current = setTimeout(() => {
      setQualityOpen(false)
      setShowControls(false)
    }, CONTROLS_HIDE_MS)
  }, [])

  useEffect(() => {
    if (!playing) {
      clearTimeout(hideTimer.current)
      setShowControls(true)
    } else {
      bumpControls()
    }
    return () => clearTimeout(hideTimer.current)
  }, [playing, video.video, bumpControls])

  // Report the exact pixel coordinates of the transparent hole to the host (phone_layers).
  useLayoutEffect(() => {
    if (remote) return
    const el = holeRef.current
    if (!el) return
    let last = ''
    const sendPlace = () => {
      const box = el.getBoundingClientRect()
      const w = Math.round(box.width)
      const h = Math.round(box.height)
      if (w < 80 || h < 60) return
      const cmd = {
        action: 'show' as const,
        x: Math.round(box.left),
        y: Math.round(box.top),
        width: w,
        height: h,
        crop: false,
      }
      const key = `${cmd.x},${cmd.y},${cmd.width},${cmd.height}`
      if (key === last) return
      last = key
      command(cmd)
    }
    sendPlace()
    const obs = new ResizeObserver(sendPlace)
    obs.observe(el)
    const settle = setTimeout(sendPlace, 180)
    return () => {
      clearTimeout(settle)
      obs.disconnect()
      command({ action: 'hide' })
    }
  }, [remote])

  const duration = mine ? state.duration : 0
  const pos = mine ? Math.min(duration || Infinity, positionNow()) : 0
  void tick
  const pct = duration > 0 ? Math.min(100, Math.max(0, (pos / duration) * 100)) : 0
  const qualities = mine ? state.qualities : []

  const togglePlay = () => {
    command({ action: playing ? 'pause' : 'play' })
    bumpControls()
  }

  const handleSeek = (e: React.PointerEvent<HTMLDivElement>) => {
    e.stopPropagation()
    if (!mine || state.live || duration <= 0) return
    const rect = e.currentTarget.getBoundingClientRect()
    const frac = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width))
    command({ action: 'seek', seconds: frac * duration })
    bumpControls()
  }

  return (
    <div className="yt-ipad-stage-wrap">
      <div
        ref={holeRef}
        className="yt-ipad-video-hole"
        onClick={() => {
          if (showControls) {
            setQualityOpen(false)
            setShowControls(false)
          } else {
            bumpControls()
          }
        }}
      >
        {(remote || buffering) && video.thumbnail && (
          <img className="yt-ipad-poster" src={video.thumbnail} alt="" />
        )}

        {showControls && (
          <div className="yt-ipad-overlay">
            <div className="yt-ipad-overlay-top">
              <div className="yt-ipad-overlay-title">{video.title}</div>
              <button
                type="button"
                className="yt-ipad-icon-btn"
                onClick={(e) => {
                  e.stopPropagation()
                  onStop()
                }}
                aria-label={words.closeVideo}
              >
                ✕
              </button>
            </div>

            <div className="yt-ipad-overlay-mid">
              <button
                type="button"
                className="yt-ipad-ctrl-btn"
                onClick={(e) => {
                  e.stopPropagation()
                  seekBy(-10)
                  bumpControls()
                }}
              >
                -10s
              </button>
              <button
                type="button"
                className="yt-ipad-ctrl-btn is-main"
                onClick={(e) => {
                  e.stopPropagation()
                  togglePlay()
                }}
              >
                {playing ? '❚❚' : '▶'}
              </button>
              <button
                type="button"
                className="yt-ipad-ctrl-btn"
                onClick={(e) => {
                  e.stopPropagation()
                  seekBy(10)
                  bumpControls()
                }}
              >
                +10s
              </button>
              <button
                type="button"
                className="yt-ipad-ctrl-btn"
                onClick={(e) => {
                  e.stopPropagation()
                  onNext()
                  bumpControls()
                }}
                aria-label={words.next}
              >
                ⏭
              </button>
            </div>

            <div className="yt-ipad-overlay-bot" onClick={(e) => e.stopPropagation()}>
              {!state?.live && duration > 0 && (
                <div className="yt-ipad-scrub" onPointerDown={handleSeek}>
                  <div className="yt-ipad-scrub-track">
                    <div className="yt-ipad-scrub-fill" style={{ width: `${pct}%` }} />
                  </div>
                </div>
              )}
              <div className="yt-ipad-bot-row">
                <span className="yt-ipad-time">
                  {state?.live ? (
                    <span className="yt-ipad-live-dot">● LIVE</span>
                  ) : (
                    `${formatClock(pos)} / ${formatClock(duration)}`
                  )}
                </span>
                <div className="yt-ipad-bot-actions">
                  <button
                    type="button"
                    className={`yt-ipad-pill-btn yt-ipad-pill-audio${audioOutput === 'device' ? ' is-device' : ''}`}
                    onClick={() => {
                      onToggleAudioOutput()
                      bumpControls()
                    }}
                    title={words.audioOutputTitle}
                  >
                    {audioOutput === 'device' ? '📱' : '🚛'} {audioLabel}
                  </button>
                  <button
                    type="button"
                    className="yt-ipad-pill-btn"
                    onClick={() => {
                      toggleMute()
                      bumpControls()
                    }}
                  >
                    {muted || volume === 0 ? '🔇' : '🔊'}
                  </button>
                  {qualities.length > 0 && (
                    <div className="yt-ipad-qual-wrap">
                      <button
                        type="button"
                        className="yt-ipad-pill-btn"
                        onClick={() => {
                          setQualityOpen((q) => !q)
                          clearTimeout(hideTimer.current)
                        }}
                      >
                        {state?.qualityLabel || (quality === 'auto' ? 'Auto' : quality)}
                      </button>
                      {qualityOpen && (
                        <div className="yt-ipad-qual-menu">
                          <button
                            type="button"
                            className={quality === 'auto' ? 'active' : ''}
                            onClick={() => {
                              setQuality('auto')
                              setQualityOpen(false)
                              bumpControls()
                            }}
                          >
                            Auto
                          </button>
                          {qualities.slice(0, 6).map((q) => (
                            <button
                              key={q.quality}
                              type="button"
                              className={quality === q.quality ? 'active' : ''}
                              onClick={() => {
                                setQuality(q.quality)
                                setQualityOpen(false)
                                bumpControls()
                              }}
                            >
                              {q.label}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Solid title bar directly under the video */}
      <div className="yt-ipad-nowbar">
        <div className="yt-ipad-nowbar-copy">
          <strong>{video.title}</strong>
          <span>{video.channel}</span>
        </div>
        <button
          type="button"
          className="yt-ipad-nowbar-btn"
          onClick={togglePlay}
        >
          {playing ? '❚❚' : '▶'}
        </button>
      </div>
    </div>
  )
}

interface Words {
  home: string
  back: string
  search: string
  account: string
  signIn: string
  signingIn: string
  signOut: string
  tabHome: string
  tabSubs: string
  tabUpNext: string
  tabHistory: string
  recentSearches: string
  continueWatching: string
  signedOutHint: string
  emptyHistory: string
  loading: string
  failed: string
  retry: string
  noResults: string
  loadMore: string
  closeVideo: string
  next: string
  audioOutputTitle: string
  audioCabin: string
  audioPhone: string
  audioIpad: string
  chipAll: string
  liveResults: string
}

interface CategoryChip {
  id: string
  label: string
  query: string
}

const WORDS: Record<'en' | 'pt', Words> = {
  en: {
    home: 'Home',
    back: 'Back',
    search: 'Search YouTube',
    account: 'Account',
    signIn: 'Sign in',
    signingIn: 'Signing in…',
    signOut: 'Sign out',
    tabHome: 'Home',
    tabSubs: 'Subscriptions',
    tabUpNext: 'Up next',
    tabHistory: 'History',
    recentSearches: 'Recent searches',
    continueWatching: 'Continue watching',
    signedOutHint: 'Sign in for your feed, or search any video ad-free.',
    emptyHistory: 'No videos watched yet.',
    loading: 'Loading…',
    failed: 'Could not load YouTube right now.',
    retry: 'Try again',
    noResults: 'No videos found.',
    loadMore: 'Load more',
    closeVideo: 'Close video',
    next: 'Next video',
    audioOutputTitle: 'Sound output (Cab speakers or Phone/iPad)',
    audioCabin: 'Cab',
    audioPhone: 'Phone',
    audioIpad: 'iPad',
    chipAll: 'All',
    liveResults: 'Videos',
  },
  pt: {
    home: 'Início',
    back: 'Voltar',
    search: 'Pesquisar no YouTube',
    account: 'Conta',
    signIn: 'Entrar',
    signingIn: 'Entrando…',
    signOut: 'Sair',
    tabHome: 'Início',
    tabSubs: 'Inscrições',
    tabUpNext: 'A seguir',
    tabHistory: 'Histórico',
    recentSearches: 'Pesquisas recentes',
    continueWatching: 'Continuar assistindo',
    signedOutHint: 'Entre para ver seu feed ou pesquise qualquer vídeo sem anúncios.',
    emptyHistory: 'Nenhum vídeo assistido ainda.',
    loading: 'Carregando…',
    failed: 'Não foi possível carregar o YouTube agora.',
    retry: 'Tentar novamente',
    noResults: 'Nenhum vídeo encontrado.',
    loadMore: 'Carregar mais',
    closeVideo: 'Fechar vídeo',
    next: 'Próximo vídeo',
    audioOutputTitle: 'Saída de som (Cabine ou Celular/iPad)',
    audioCabin: 'Cabine',
    audioPhone: 'Celular',
    audioIpad: 'iPad',
    chipAll: 'Tudo',
    liveResults: 'Vídeos',
  },
}

const CATEGORY_CHIPS: Record<'en' | 'pt', CategoryChip[]> = {
  pt: [
    { id: 'music', label: 'Música', query: 'músicas mais tocadas Brasil' },
    { id: 'games', label: 'Jogos', query: 'gameplay jogos Brasil' },
    { id: 'ets2', label: 'Euro Truck Simulator 2', query: 'Euro Truck Simulator 2 Brasil' },
    { id: 'podcasts', label: 'Podcasts', query: 'podcast completo Brasil' },
    { id: 'mixes', label: 'Mixes', query: 'mix músicas estrada' },
    { id: 'forro', label: 'Forró', query: 'forró e piseiro 2026' },
    { id: 'sertanejo', label: 'Música sertaneja', query: 'modão sertanejo universitário' },
    { id: 'live', label: 'Ao vivo', query: 'ao vivo agora Brasil' },
    { id: 'hiphop', label: 'Hip hop brasileiro', query: 'rap trap hip hop brasileiro' },
    { id: 'trucks', label: 'Caminhões', query: 'caminhões qualificados estrada Brasil' },
  ],
  en: [
    { id: 'music', label: 'Music', query: 'top music hits playlist' },
    { id: 'games', label: 'Gaming', query: 'gaming highlights gameplay' },
    { id: 'ets2', label: 'Euro Truck Simulator 2', query: 'Euro Truck Simulator 2 gameplay' },
    { id: 'podcasts', label: 'Podcasts', query: 'full podcast episodes' },
    { id: 'mixes', label: 'Mixes', query: 'road trip music mix' },
    { id: 'country', label: 'Country', query: 'country highway trucking songs' },
    { id: 'live', label: 'Live', query: 'live streams right now' },
    { id: 'lofi', label: 'Lo-fi', query: 'lofi hip hop beats radio' },
    { id: 'trucks', label: 'Trucks', query: 'semi trucks highway convoy' },
  ],
}

const QUICK_TOPICS: Record<'en' | 'pt', string[]> = {
  en: ['ETS2 Trucking', 'Lo-fi Radio', 'Trucker Country', 'Scania V8', 'Live News'],
  pt: ['ETS2 Brasil', 'Modão Sertanejo', 'Scania V8', 'Músicas para Estrada', 'Notícias ao vivo'],
}
