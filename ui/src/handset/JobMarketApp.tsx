import { useCallback, useEffect, useMemo, useState } from 'react'
import { TextField } from '../cabin/keyboard/TextField'
import { useLive } from '../cabin/state/store'
import type { JobMarketAnswer, JobMarketMode, JobMarketOffer, JobMarketSnapshot } from '../shared/host'
import { Segmented } from './ios'
import { acceptJobOffer, cancelActiveJob, fetchCargoIcon, refreshJobMarket } from './jobMarketBridge'
import { cargoFallbackPhoto } from './cargoPhotos'
import { distance, formatMoney, type Prefs } from './prefs'

type SortKey = 'nearest' | 'reward' | 'rate' | 'distance'

const PAGE_SIZE = 25

export interface JobMarketWords {
  tabFreight: string
  tabCargo: string
  tabQuick: string
  searchPlaceholder: string
  refresh: string
  loading: string
  sortNearest: string
  sortReward: string
  sortRate: string
  sortDistance: string
  allCities: string
  nearbyOnly: string
  offersCount: (shown: number, total: number) => string
  details: string
  backToList: string
  accept: string
  acceptJob: string
  accepting: string
  confirmQuick: string
  confirmYes: string
  confirmCancel: string
  showMore: (remaining: number) => string
  fromYou: (dist: string) => string
  expiresIn: (text: string) => string
  ferry: string
  urgent: string
  important: string
  normal: string
  fragile: string
  valuable: string
  overweight: string
  oversize: string
  adr: (cls: number) => string
  estCargoHint: string
  bannerOnJob: string
  bannerNoTruck: string
  bannerNoTrailer: string
  bannerQuickNote: string
  emptyBoard: string
  emptyFilter: string
  emptyNoGame: string
  emptyMultiplayer: string
  emptyUnsupported: string
  emptyNotReady: string
  emptyError: string
  toastAccepted: string
  toastAcceptedQuick: string
  toastOnJob: string
  toastNoTruck: string
  toastNoTrailer: string
  toastExpired: string
  toastMultiplayer: string
  toastUnsupported: string
  toastFailed: string
  perKm: string
  perMi: string
  detailOrigin: string
  detailDest: string
  detailDistance: string
  detailWeight: string
  detailUnits: string
  detailRate: string
  detailUrgency: string
  detailFragility: string
  detailExpires: string
  detailFerry: string
  detailTrailer: string
  detailTruck: string
  detailOwnTrailer: string
  detailLoadingImg: string
  detailNoImg: string
  activeJobTitle: string
  activePickingUp: string
  activeDelivering: string
  activeView: string
  activeHide: string
  cancelJob: string
  cancellingJob: string
  cancelConfirm: string
  cancelYes: string
  cancelKeep: string
  toastCancelled: string
  toastCancelFailed: string
}

export const JOB_MARKET_WORDS: Record<'en' | 'pt', JobMarketWords> = {
  pt: {
    tabFreight: 'Fretes',
    tabCargo: 'Cargas',
    tabQuick: 'Rápido',
    searchPlaceholder: 'Buscar cidade, carga ou empresa…',
    refresh: 'Atualizar',
    loading: 'Lendo fretes do jogo…',
    sortNearest: 'Perto',
    sortReward: 'Valor',
    sortRate: 'Por km',
    sortDistance: 'Dist.',
    allCities: 'Todas',
    nearbyOnly: 'Perto',
    offersCount: (shown, total) => (shown < total ? `${shown} de ${total} ofertas` : `${total} ofertas`),
    details: 'Ver detalhes',
    backToList: 'Voltar às ofertas',
    accept: 'Aceitar',
    acceptJob: 'Aceitar frete',
    accepting: 'Aceitando…',
    confirmQuick: 'Trocar para o caminhão da empresa e iniciar este frete?',
    confirmYes: 'Confirmar',
    confirmCancel: 'Cancelar',
    showMore: (rem) => `Mostrar mais (${rem} restantes)`,
    fromYou: (d) => `a ${d}`,
    expiresIn: (t) => `expira em ${t}`,
    ferry: 'Balsa',
    urgent: 'Urgente',
    important: 'Prioritário',
    normal: 'Normal',
    fragile: 'Frágil',
    valuable: 'Valiosa',
    overweight: 'Pesada',
    oversize: 'Especial',
    adr: (c) => `ADR ${c}`,
    estCargoHint: 'Peso e valor estimados para sua carreta própria.',
    bannerOnJob: 'Você já está em um frete ativo. Finalize ou cancele antes de aceitar outro.',
    bannerNoTruck: 'Você precisa de caminhão próprio para Fretes. Use a aba Rápido.',
    bannerNoTrailer: 'Engate sua carreta própria para aceitar cargas neste mercado.',
    bannerQuickNote: 'Trabalho Rápido usa caminhão da empresa e leva você direto para o local.',
    emptyBoard: 'Nenhuma oferta disponível nesta aba agora.',
    emptyFilter: 'Nenhuma oferta encontrada para este filtro.',
    emptyNoGame: 'Inicie o ETS2 ou ATS e entre na cabine para carregar os fretes ao vivo.',
    emptyMultiplayer: 'A Bolsa de Fretes ao vivo fica bloqueada no TruckersMP.',
    emptyUnsupported: 'Versão do jogo ainda não reconhecida — desativado por segurança.',
    emptyNotReady: 'Economia do jogo ainda carregando. Entre na cabine e toque em Atualizar.',
    emptyError: 'Não foi possível ler os fretes agora. Tente atualizar novamente.',
    toastAccepted: 'Frete aceito! Siga o GPS até a empresa para carregar.',
    toastAcceptedQuick: 'Trabalho rápido aceito! Preparando caminhão da empresa…',
    toastOnJob: 'Você já tem um frete ativo no jogo.',
    toastNoTruck: 'É necessário ter um caminhão próprio.',
    toastNoTrailer: 'Engate sua carreta própria primeiro.',
    toastExpired: 'Esta oferta expirou ou mudou. Atualizando lista…',
    toastMultiplayer: 'Bloqueado no TruckersMP.',
    toastUnsupported: 'Indisponível nesta versão do jogo.',
    toastFailed: 'O jogo recusou a oferta. Tente atualizar a lista.',
    perKm: '/km',
    perMi: '/mi',
    detailOrigin: 'Origem',
    detailDest: 'Destino',
    detailDistance: 'Distância',
    detailWeight: 'Peso',
    detailUnits: 'Unidades',
    detailRate: 'Valor por dist.',
    detailUrgency: 'Urgência',
    detailFragility: 'Fragilidade',
    detailExpires: 'Oferta expira em',
    detailFerry: 'Travessia de balsa',
    detailTrailer: 'Carreta',
    detailTruck: 'Caminhão da empresa',
    detailOwnTrailer: 'Carreta própria acoplada',
    detailLoadingImg: 'Carregando imagem da carga…',
    detailNoImg: 'Sem miniatura no mod',
    activeJobTitle: 'Frete atual',
    activePickingUp: 'Indo coletar',
    activeDelivering: 'Em entrega',
    activeView: 'Ver frete atual',
    activeHide: 'Ocultar',
    cancelJob: 'Cancelar frete',
    cancellingJob: 'Cancelando…',
    cancelConfirm: 'Cancelar este frete agora? O jogo pode cobrar a multa de cancelamento.',
    cancelYes: 'Sim, cancelar',
    cancelKeep: 'Manter frete',
    toastCancelled: 'Frete cancelado! Você já pode aceitar outro.',
    toastCancelFailed: 'Não foi possível cancelar o frete agora.',
  },
  en: {
    tabFreight: 'Freight',
    tabCargo: 'Cargo',
    tabQuick: 'Quick',
    searchPlaceholder: 'Search city, cargo or company…',
    refresh: 'Refresh',
    loading: 'Reading live game jobs…',
    sortNearest: 'Near',
    sortReward: 'Pay',
    sortRate: 'Rate',
    sortDistance: 'Dist.',
    allCities: 'All',
    nearbyOnly: 'Nearby',
    offersCount: (shown, total) => (shown < total ? `${shown} of ${total} offers` : `${total} offers`),
    details: 'Details',
    backToList: 'Back to offers',
    accept: 'Accept',
    acceptJob: 'Accept job',
    accepting: 'Accepting…',
    confirmQuick: 'Switch to the company truck and start this Quick Job?',
    confirmYes: 'Confirm',
    confirmCancel: 'Cancel',
    showMore: (rem) => `Show more (${rem} left)`,
    fromYou: (d) => `${d} away`,
    expiresIn: (t) => `expires in ${t}`,
    ferry: 'Ferry',
    urgent: 'Urgent',
    important: 'Priority',
    normal: 'Standard',
    fragile: 'Fragile',
    valuable: 'Valuable',
    overweight: 'Heavy',
    oversize: 'Oversize',
    adr: (c) => `ADR ${c}`,
    estCargoHint: 'Weight and payout estimated for your owned trailer.',
    bannerOnJob: 'You already have an active job. Finish or cancel it before taking another.',
    bannerNoTruck: 'You need your own truck for Freight Market. Try the Quick tab.',
    bannerNoTrailer: 'Attach your owned trailer to accept Cargo Market offers.',
    bannerQuickNote: 'Quick Jobs provide a company truck and move you straight to the job.',
    emptyBoard: 'No offers available in this market right now.',
    emptyFilter: 'No offers match your filter.',
    emptyNoGame: 'Start ETS2 or ATS and enter the cab to load live jobs.',
    emptyMultiplayer: 'Live Job Market is disabled on TruckersMP.',
    emptyUnsupported: 'Game version not recognized — safely disabled.',
    emptyNotReady: 'Game economy is still loading. Enter the cab and tap Refresh.',
    emptyError: 'Could not read live jobs right now. Try refreshing.',
    toastAccepted: 'Job accepted! Follow the GPS to the company yard.',
    toastAcceptedQuick: 'Quick job accepted! Loading company truck…',
    toastOnJob: 'You already have an active job.',
    toastNoTruck: 'You need your own truck.',
    toastNoTrailer: 'Attach your owned trailer first.',
    toastExpired: 'That offer expired. Refreshing list…',
    toastMultiplayer: 'Disabled in TruckersMP.',
    toastUnsupported: 'Unavailable on this game version.',
    toastFailed: 'The game declined this offer. Try refreshing.',
    perKm: '/km',
    perMi: '/mi',
    detailOrigin: 'Origin',
    detailDest: 'Destination',
    detailDistance: 'Distance',
    detailWeight: 'Weight',
    detailUnits: 'Units',
    detailRate: 'Pay rate',
    detailUrgency: 'Urgency',
    detailFragility: 'Fragility',
    detailExpires: 'Offer expires in',
    detailFerry: 'Ferry crossing',
    detailTrailer: 'Trailer',
    detailTruck: 'Company truck',
    detailOwnTrailer: 'Your own trailer',
    detailLoadingImg: 'Loading cargo image…',
    detailNoImg: 'No preview image',
    activeJobTitle: 'Active job',
    activePickingUp: 'Picking up',
    activeDelivering: 'Delivering',
    activeView: 'View active job',
    activeHide: 'Hide',
    cancelJob: 'Cancel job',
    cancellingJob: 'Cancelling…',
    cancelConfirm: 'Cancel this job now? The game may charge a cancellation penalty.',
    cancelYes: 'Yes, cancel',
    cancelKeep: 'Keep job',
    toastCancelled: 'Job cancelled! You can accept another offer now.',
    toastCancelFailed: 'Could not cancel the active job right now.',
  },
}

export function JobMarketApp({
  words,
  units,
  currency,
  language,
  remote = false,
}: {
  words: JobMarketWords
  units: Prefs['units']
  currency: Prefs['currency']
  language: 'en' | 'pt'
  remote?: boolean
}) {
  const gameCurrency = useLive((state) => state.wallet?.currency ?? 'EUR')
  const liveJob = useLive((state) => state.job)
  const liveJobActive = liveJob !== null
  const navDistance = useLive((state) => state.telemetry?.navigationDistance ?? 0)
  const gameTime = useLive((state) => state.telemetry?.gameTime ?? null)
  const gameRunning = useLive((state) => state.telemetry !== null)

  const [mode, setMode] = useState<JobMarketMode>('freight')
  const [sort, setSort] = useState<SortKey>('nearest')
  const [query, setQuery] = useState('')
  const [cityFilter, setCityFilter] = useState<string>('') // '' = all, '__near__' = nearby, or originCityId
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)

  const [snapshot, setSnapshot] = useState<JobMarketSnapshot | null>(null)
  const [errorCode, setErrorCode] = useState<JobMarketAnswer | null>(null)
  const [loading, setLoading] = useState(false)
  const [busyKey, setBusyKey] = useState<string | null>(null)
  const [confirmKey, setConfirmKey] = useState<string | null>(null)
  const [selected, setSelected] = useState<JobMarketOffer | null>(null)
  const [activeOpen, setActiveOpen] = useState(true)
  const [confirmCancel, setConfirmCancel] = useState(false)
  const [cancelling, setCancelling] = useState(false)
  const [toast, setToast] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null)

  const showToast = useCallback((kind: 'ok' | 'err', text: string) => {
    setToast({ kind, text })
  }, [])

  useEffect(() => {
    if (!toast) return
    const t = window.setTimeout(() => setToast(null), 4500)
    return () => window.clearTimeout(t)
  }, [toast])

  const load = useCallback(async () => {
    setLoading(true)
    setConfirmKey(null)
    const res = await refreshJobMarket()
    setLoading(false)
    if (res.answer === 'ok' && res.data) {
      setSnapshot(res.data)
      setErrorCode(null)
      setVisibleCount(PAGE_SIZE)
    } else {
      setErrorCode(res.answer)
      if (res.answer === 'multiplayer' || res.answer === 'unsupported') {
        setSnapshot(null)
      }
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const modeOffers = useMemo(() => {
    if (!snapshot) return []
    return snapshot.offers.filter((o) => o.mode === mode)
  }, [snapshot, mode])

  const quickCities = useMemo(() => {
    const map = new Map<string, { id: string; name: string; dist: number; count: number }>()
    for (const o of modeOffers) {
      const prev = map.get(o.originCityId)
      const d = o.distFromPlayerKm ?? 999_999
      if (!prev) {
        map.set(o.originCityId, { id: o.originCityId, name: o.originCity, dist: d, count: 1 })
      } else {
        prev.count += 1
        if (d < prev.dist) prev.dist = d
      }
    }
    return [...map.values()].sort((a, b) => a.dist - b.dist || b.count - a.count).slice(0, 5)
  }, [modeOffers])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    const list = modeOffers.filter((o) => {
      if (cityFilter === '__near__') {
        if ((o.distFromPlayerKm ?? 999_999) > 150) return false
      } else if (cityFilter) {
        if (o.originCityId !== cityFilter) return false
      }
      if (!q) return true
      return (
        o.cargoName.toLowerCase().includes(q) ||
        o.originCity.toLowerCase().includes(q) ||
        o.destCity.toLowerCase().includes(q) ||
        o.originCompany.toLowerCase().includes(q) ||
        o.destCompany.toLowerCase().includes(q)
      )
    })

    const sorted = [...list]
    sorted.sort((a, b) => {
      if (sort === 'nearest') {
        const da = a.distFromPlayerKm ?? 999_999
        const db = b.distFromPlayerKm ?? 999_999
        if (da !== db) return da - db
        return b.reward - a.reward
      }
      if (sort === 'reward') return b.reward - a.reward
      if (sort === 'rate') {
        const ra = a.distanceKm > 0 ? a.reward / a.distanceKm : 0
        const rb = b.distanceKm > 0 ? b.reward / b.distanceKm : 0
        return rb - ra
      }
      return a.distanceKm - b.distanceKm
    })
    return sorted
  }, [modeOffers, query, cityFilter, sort])

  const shownOffers = filtered.slice(0, visibleCount)
  const remaining = Math.max(0, filtered.length - shownOffers.length)

  const onJob = (snapshot?.onJob ?? false) || liveJobActive
  const canAcceptFor = (m: JobMarketMode) =>
    !onJob &&
    (m === 'quick'
      ? (snapshot?.quickSupported ?? false)
      : m === 'freight'
        ? (snapshot?.hasOwnTruck ?? true)
        : (snapshot?.hasOwnTruck ?? true) && (snapshot?.trailerAttached ?? false))

  const handleAccept = async (offer: JobMarketOffer) => {
    const key = `${offer.mode}:${offer.companyPtr}:${offer.offerPtr}`
    if (offer.mode === 'quick' && confirmKey !== key) {
      setConfirmKey(key)
      return
    }
    setConfirmKey(null)
    setBusyKey(key)
    const ans = await acceptJobOffer(offer.mode, offer.companyPtr, offer.offerPtr)
    setBusyKey(null)

    switch (ans) {
      case 'ok':
        showToast('ok', offer.mode === 'quick' ? words.toastAcceptedQuick : words.toastAccepted)
        setSelected(null)
        window.setTimeout(() => void load(), 700)
        break
      case 'onJob':
        showToast('err', words.toastOnJob)
        break
      case 'noTruck':
        showToast('err', words.toastNoTruck)
        break
      case 'noTrailer':
        showToast('err', words.toastNoTrailer)
        break
      case 'offerExpired':
        showToast('err', words.toastExpired)
        setSelected(null)
        void load()
        break
      case 'multiplayer':
        showToast('err', words.toastMultiplayer)
        break
      case 'unsupported':
        showToast('err', words.toastUnsupported)
        break
      default:
        showToast('err', words.toastFailed)
        break
    }
  }

  const handleCancelActive = async () => {
    if (!confirmCancel) {
      setConfirmCancel(true)
      return
    }
    setCancelling(true)
    const ans = await cancelActiveJob()
    setCancelling(false)
    setConfirmCancel(false)
    if (ans === 'ok') {
      showToast('ok', words.toastCancelled)
      window.setTimeout(() => void load(), 600)
    } else if (ans === 'multiplayer') {
      showToast('err', words.toastMultiplayer)
    } else if (ans === 'unsupported') {
      showToast('err', words.toastUnsupported)
    } else {
      showToast('err', words.toastCancelFailed)
    }
  }

  if (selected) {
    const key = `${selected.mode}:${selected.companyPtr}:${selected.offerPtr}`
    return (
      <OfferDetailView
        offer={selected}
        words={words}
        units={units}
        currency={currency}
        gameCurrency={gameCurrency}
        language={language}
        canAccept={canAcceptFor(selected.mode)}
        isBusy={busyKey === key}
        isConfirming={confirmKey === key}
        toast={toast}
        onBack={() => {
          setConfirmKey(null)
          setSelected(null)
        }}
        onCancelConfirm={() => setConfirmKey(null)}
        onAccept={() => void handleAccept(selected)}
      />
    )
  }

  const tabs: [JobMarketMode, string][] =
    snapshot?.quickSupported === false
      ? [
          ['freight', words.tabFreight],
          ['cargo', words.tabCargo],
        ]
      : [
          ['freight', words.tabFreight],
          ['cargo', words.tabCargo],
          ['quick', words.tabQuick],
        ]

  const canAcceptMode = canAcceptFor(mode)

  return (
    <div className="jm-root">
      <div className="jm-controls">
        <Segmented
          options={tabs}
          chosen={mode}
          choose={(next) => {
            setMode(next as JobMarketMode)
            setCityFilter('')
            setConfirmKey(null)
            setVisibleCount(PAGE_SIZE)
          }}
        />

        <div className="jm-search-row">
          {remote ? (
            <input
              type="search"
              className="jm-search"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value)
                setVisibleCount(PAGE_SIZE)
              }}
              placeholder={words.searchPlaceholder}
            />
          ) : (
            <div className="jm-search-field">
              <TextField
                id="job-market-search"
                value={query}
                onChange={(next) => {
                  setQuery(next)
                  setVisibleCount(PAGE_SIZE)
                }}
                returnKind="search"
                hint={words.searchPlaceholder}
                label={words.searchPlaceholder}
                icon={
                  <svg viewBox="0 0 24 24" width="14" height="14" className="jm-search-icon" aria-hidden>
                    <circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" strokeWidth="2.2" />
                    <path d="m16.2 16.2 4.3 4.3" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
                  </svg>
                }
              />
            </div>
          )}
          <button
            type="button"
            className={`jm-refresh${loading ? ' spinning' : ''}`}
            onClick={() => void load()}
            disabled={loading}
            aria-label={words.refresh}
            title={words.refresh}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden>
              <path
                d="M20 12a8 8 0 1 1-2.3-5.6L20 8.5M20 4v4.5h-4.5"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>

        <div className="jm-sort-row">
          {(
            [
              ['nearest', words.sortNearest],
              ['reward', words.sortReward],
              ['rate', words.sortRate],
              ['distance', words.sortDistance],
            ] as [SortKey, string][]
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              className={`jm-pill${sort === key ? ' on' : ''}`}
              onClick={() => setSort(key)}
            >
              {label}
            </button>
          ))}
        </div>

        {quickCities.length > 0 && (
          <div className="jm-city-row">
            <button
              type="button"
              className={`jm-chip${cityFilter === '' ? ' on' : ''}`}
              onClick={() => {
                setCityFilter('')
                setVisibleCount(PAGE_SIZE)
              }}
            >
              {words.allCities}
            </button>
            {quickCities[0].dist < 999_999 && (
              <button
                type="button"
                className={`jm-chip${cityFilter === '__near__' ? ' on' : ''}`}
                onClick={() => {
                  setCityFilter('__near__')
                  setVisibleCount(PAGE_SIZE)
                }}
              >
                {words.nearbyOnly}
              </button>
            )}
            {quickCities.map((c) => (
              <button
                key={c.id}
                type="button"
                className={`jm-chip${cityFilter === c.id ? ' on' : ''}`}
                onClick={() => {
                  setCityFilter(c.id)
                  setVisibleCount(PAGE_SIZE)
                }}
              >
                {c.name}
              </button>
            ))}
          </div>
        )}
      </div>

      {toast && <div className={`jm-toast ${toast.kind}`}>{toast.text}</div>}

      {onJob && (
        <section className="jm-active-card">
          <div className="jm-active-head">
            <div className="jm-active-head-left">
              <span className="jm-active-badge">
                {liveJob ? (liveJob.cargoLoaded ? words.activeDelivering : words.activePickingUp) : words.activeJobTitle}
              </span>
              <strong className="jm-active-name">{liveJob ? liveJob.cargo : words.bannerOnJob}</strong>
            </div>
            {liveJob && (
              <button
                type="button"
                className="jm-active-toggle"
                onClick={() => setActiveOpen((v) => !v)}
              >
                {activeOpen ? words.activeHide : words.activeView}
              </button>
            )}
          </div>

          {liveJob && activeOpen && (
            <>
              <div className="jm-active-body">
                <div className="jm-active-thumb">
                  <img
                    src={cargoFallbackPhoto({
                      cargoId: liveJob.cargo,
                      cargoName: liveJob.cargo,
                      adrClass: 0,
                      overweight: liveJob.cargoMass >= 26_000,
                      oversize: liveJob.special,
                      fragile: false,
                    })}
                    alt={liveJob.cargo}
                    draggable={false}
                  />
                </div>
                <div className="jm-active-info">
                  <div className="jm-active-pay">
                    {formatMoney(liveJob.income, gameCurrency, currency, language)}
                  </div>
                  <div className="jm-stop">
                    <span className="jm-dot origin" />
                    <span className="jm-city">{liveJob.sourceCity}</span>
                    {!liveJob.special && <span className="jm-company">{liveJob.sourceCompany}</span>}
                  </div>
                  <div className="jm-stop">
                    <span className="jm-dot dest" />
                    <span className="jm-city">{liveJob.destinationCity}</span>
                    {!liveJob.special && <span className="jm-company">{liveJob.destinationCompany}</span>}
                  </div>
                  <div className="jm-meta">
                    <span>
                      {navDistance > 0
                        ? distance(navDistance, units)
                        : distance(liveJob.plannedDistanceKm * 1000, units)}
                    </span>
                    <span>·</span>
                    <span>{formatMass(liveJob.cargoMass, units, language)}</span>
                    {gameTime !== null && liveJob.deliveryTime > gameTime && (
                      <>
                        <span>·</span>
                        <span>{formatDuration(liveJob.deliveryTime - gameTime)}</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            </>
          )}

          {confirmCancel ? (
            <div className="jm-confirm jm-confirm-danger">
              <span>{words.cancelConfirm}</span>
              <div className="jm-confirm-actions">
                <button
                  type="button"
                  className="jm-btn cancel"
                  disabled={cancelling}
                  onClick={() => setConfirmCancel(false)}
                >
                  {words.cancelKeep}
                </button>
                <button
                  type="button"
                  className="jm-btn danger"
                  disabled={cancelling}
                  onClick={() => void handleCancelActive()}
                >
                  {cancelling ? words.cancellingJob : words.cancelYes}
                </button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              className="jm-cancel-btn"
              disabled={cancelling}
              onClick={() => void handleCancelActive()}
            >
              {cancelling ? words.cancellingJob : words.cancelJob}
            </button>
          )}
        </section>
      )}
      {snapshot && !onJob && mode === 'freight' && !snapshot.hasOwnTruck && (
        <div className="jm-banner info">{words.bannerNoTruck}</div>
      )}
      {snapshot && !onJob && mode === 'cargo' && !snapshot.trailerAttached && (
        <div className="jm-banner info">{words.bannerNoTrailer}</div>
      )}
      {snapshot && !onJob && mode === 'quick' && <div className="jm-banner note">{words.bannerQuickNote}</div>}

      {loading && !snapshot ? (
        <div className="jm-empty">{words.loading}</div>
      ) : errorCode && !snapshot ? (
        <div className="jm-empty">
          <p>{emptyMessage(errorCode, gameRunning, words)}</p>
          <button type="button" className="jm-retry" onClick={() => void load()}>
            {words.refresh}
          </button>
        </div>
      ) : filtered.length === 0 ? (
        <div className="jm-empty">{modeOffers.length === 0 ? words.emptyBoard : words.emptyFilter}</div>
      ) : (
        <>
          <div className="jm-count">
            <span>{words.offersCount(shownOffers.length, filtered.length)}</span>
            {mode === 'cargo' && <span className="jm-est">{words.estCargoHint}</span>}
          </div>

          <div className="jm-list">
            {shownOffers.map((offer) => {
              const key = `${offer.mode}:${offer.companyPtr}:${offer.offerPtr}`
              const isBusy = busyKey === key
              const isConfirming = confirmKey === key
              const rateDist = units === 'mi' ? offer.distanceKm * 0.621_371 : offer.distanceKm
              const rateValue = rateDist > 0 ? offer.reward / rateDist : 0
              const massText = formatMass(offer.massKg, units, language)

              return (
                <article className="jm-card" key={key}>
                  <div className="jm-card-top">
                    <div className="jm-cargo">{offer.cargoName}</div>
                    <div className="jm-pay">{formatMoney(offer.reward, gameCurrency, currency, language)}</div>
                  </div>

                  <div className="jm-route">
                    <div className="jm-stop">
                      <span className="jm-dot origin" />
                      <span className="jm-city">{offer.originCity}</span>
                      <span className="jm-co">{offer.originCompany}</span>
                    </div>
                    <div className="jm-stop">
                      <span className="jm-dot dest" />
                      <span className="jm-city">{offer.destCity}</span>
                      <span className="jm-co">{offer.destCompany}</span>
                    </div>
                  </div>

                  <div className="jm-meta">
                    <span>{distance(offer.distanceKm * 1000, units)}</span>
                    <span>·</span>
                    <span>{massText}</span>
                    <span>·</span>
                    <span>
                      {formatMoney(rateValue, gameCurrency, currency, language)}
                      {units === 'mi' ? words.perMi : words.perKm}
                    </span>
                    {offer.distFromPlayerKm !== undefined && (
                      <>
                        <span>·</span>
                        <span className="jm-near">
                          {words.fromYou(distance(offer.distFromPlayerKm * 1000, units))}
                        </span>
                      </>
                    )}
                  </div>

                  <div className="jm-tags">
                    <OfferTags offer={offer} words={words} />
                    <span className="jm-exp">{words.expiresIn(formatDuration(offer.expiresInMin))}</span>
                  </div>

                  {isConfirming ? (
                    <div className="jm-confirm">
                      <span>{words.confirmQuick}</span>
                      <div className="jm-confirm-actions">
                        <button type="button" className="jm-btn cancel" onClick={() => setConfirmKey(null)}>
                          {words.confirmCancel}
                        </button>
                        <button
                          type="button"
                          className="jm-btn primary"
                          disabled={isBusy}
                          onClick={() => void handleAccept(offer)}
                        >
                          {words.confirmYes}
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="jm-card-actions">
                      <button
                        type="button"
                        className="jm-details-btn"
                        onClick={() => {
                          setConfirmKey(null)
                          setSelected(offer)
                        }}
                      >
                        {words.details}
                      </button>
                      <button
                        type="button"
                        className="jm-accept"
                        disabled={!canAcceptMode || busyKey !== null}
                        onClick={() => void handleAccept(offer)}
                      >
                        {isBusy ? words.accepting : words.accept}
                      </button>
                    </div>
                  )}
                </article>
              )
            })}
          </div>

          {remaining > 0 && (
            <button
              type="button"
              className="jm-more"
              onClick={() => setVisibleCount((c) => c + PAGE_SIZE)}
            >
              {words.showMore(remaining)}
            </button>
          )}
        </>
      )}
    </div>
  )
}

function OfferDetailView({
  offer,
  words,
  units,
  currency,
  gameCurrency,
  language,
  canAccept,
  isBusy,
  isConfirming,
  toast,
  onBack,
  onCancelConfirm,
  onAccept,
}: {
  offer: JobMarketOffer
  words: JobMarketWords
  units: Prefs['units']
  currency: Prefs['currency']
  gameCurrency: 'USD' | 'EUR'
  language: 'en' | 'pt'
  canAccept: boolean
  isBusy: boolean
  isConfirming: boolean
  toast: { kind: 'ok' | 'err'; text: string } | null
  onBack: () => void
  onCancelConfirm: () => void
  onAccept: () => void
}) {
  const [gameIconUrl, setGameIconUrl] = useState<string | null>(null)
  const fallbackPhoto = cargoFallbackPhoto(offer)

  useEffect(() => {
    let alive = true
    setGameIconUrl(null)
    void fetchCargoIcon(offer.cargoIcon ?? '', offer.cargoId).then((url) => {
      if (!alive) return
      setGameIconUrl(url)
    })
    return () => {
      alive = false
    }
  }, [offer.cargoIcon, offer.cargoId])

  const rateDist = units === 'mi' ? offer.distanceKm * 0.621_371 : offer.distanceKm
  const rateValue = rateDist > 0 ? offer.reward / rateDist : 0
  const urgencyLabel = offer.urgency >= 2 ? words.urgent : offer.urgency === 1 ? words.important : words.normal

  return (
    <div className="jm-detail">
      <button type="button" className="jm-back-btn" onClick={onBack}>
        <svg viewBox="0 0 12 20" width="9" height="14" aria-hidden>
          <path d="M10 2 2 10l8 8" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" />
        </svg>
        {words.backToList}
      </button>

      {toast && <div className={`jm-toast ${toast.kind}`}>{toast.text}</div>}

      <div className="jm-detail-hero">
        <div className="jm-detail-img-wrap">
          <img
            src={gameIconUrl ?? fallbackPhoto}
            alt={offer.cargoName}
            className={gameIconUrl ? 'jm-detail-img is-game-icon' : 'jm-detail-img is-photo'}
            draggable={false}
          />
        </div>
        <div className="jm-detail-head">
          <div className="jm-detail-title">{offer.cargoName}</div>
          <div className="jm-detail-pay">{formatMoney(offer.reward, gameCurrency, currency, language)}</div>
        </div>
        <div className="jm-tags">
          <OfferTags offer={offer} words={words} />
        </div>
      </div>

      <div className="jm-detail-card">
        <div className="jm-detail-stop">
          <span className="jm-dot origin" />
          <div className="jm-detail-stop-text">
            <span className="jm-detail-kicker">
              {words.detailOrigin}
              {offer.distFromPlayerKm !== undefined
                ? ` · ${words.fromYou(distance(offer.distFromPlayerKm * 1000, units))}`
                : ''}
            </span>
            <strong>{offer.originCity}</strong>
            <span>{offer.originCompany}</span>
          </div>
        </div>
        <div className="jm-detail-divider" />
        <div className="jm-detail-stop">
          <span className="jm-dot dest" />
          <div className="jm-detail-stop-text">
            <span className="jm-detail-kicker">{words.detailDest}</span>
            <strong>{offer.destCity}</strong>
            <span>{offer.destCompany}</span>
          </div>
        </div>
      </div>

      <div className="jm-detail-grid">
        <div className="jm-detail-cell">
          <span>{words.detailDistance}</span>
          <strong>{distance(offer.distanceKm * 1000, units)}</strong>
        </div>
        <div className="jm-detail-cell">
          <span>{words.detailWeight}</span>
          <strong>{formatMass(offer.massKg, units, language)}</strong>
        </div>
        <div className="jm-detail-cell">
          <span>{words.detailRate}</span>
          <strong>
            {formatMoney(rateValue, gameCurrency, currency, language)}
            {units === 'mi' ? words.perMi : words.perKm}
          </strong>
        </div>
        <div className="jm-detail-cell">
          <span>{words.detailUrgency}</span>
          <strong>{urgencyLabel}</strong>
        </div>
        {offer.unitsCount !== undefined && offer.unitsCount > 0 && (
          <div className="jm-detail-cell">
            <span>{words.detailUnits}</span>
            <strong>{offer.unitsCount}×</strong>
          </div>
        )}
        <div className="jm-detail-cell">
          <span>{words.detailFragility}</span>
          <strong>{offer.fragilityPct ?? (offer.fragile ? 50 : 0)}%</strong>
        </div>
        <div className="jm-detail-cell">
          <span>{words.detailExpires}</span>
          <strong>{formatDuration(offer.expiresInMin)}</strong>
        </div>
        {offer.ferryTime > 0 && (
          <div className="jm-detail-cell">
            <span>{words.detailFerry}</span>
            <strong>
              {formatDuration(offer.ferryTime)} · {formatMoney(offer.ferryPrice, gameCurrency, currency, language)}
            </strong>
          </div>
        )}
        {offer.mode === 'quick' && offer.companyTruck && (
          <div className="jm-detail-cell wide">
            <span>{words.detailTruck}</span>
            <strong>{formatVehicleId(offer.companyTruck)}</strong>
          </div>
        )}
        {offer.mode !== 'cargo' && offer.trailerDef && (
          <div className="jm-detail-cell wide">
            <span>{words.detailTrailer}</span>
            <strong>{formatTrailerDef(offer.trailerDef)}</strong>
          </div>
        )}
        {offer.mode === 'cargo' && (
          <div className="jm-detail-cell wide">
            <span>{words.detailTrailer}</span>
            <strong>{words.detailOwnTrailer}</strong>
          </div>
        )}
      </div>

      {isConfirming ? (
        <div className="jm-confirm">
          <span>{words.confirmQuick}</span>
          <div className="jm-confirm-actions">
            <button type="button" className="jm-btn cancel" onClick={onCancelConfirm}>
              {words.confirmCancel}
            </button>
            <button type="button" className="jm-btn primary" disabled={isBusy} onClick={onAccept}>
              {words.confirmYes}
            </button>
          </div>
        </div>
      ) : (
        <button type="button" className="jm-accept jm-accept-lg" disabled={!canAccept || isBusy} onClick={onAccept}>
          {isBusy ? words.accepting : words.acceptJob}
        </button>
      )}
    </div>
  )
}

function OfferTags({ offer, words }: { offer: JobMarketOffer; words: JobMarketWords }) {
  return (
    <>
      {offer.urgency >= 2 && <span className="jm-tag red">{words.urgent}</span>}
      {offer.urgency === 1 && <span className="jm-tag amber">{words.important}</span>}
      {offer.fragile && <span className="jm-tag blue">{words.fragile}</span>}
      {offer.valuable && <span className="jm-tag purple">{words.valuable}</span>}
      {offer.adrClass > 0 && <span className="jm-tag orange">{words.adr(offer.adrClass)}</span>}
      {offer.overweight && <span className="jm-tag slate">{words.overweight}</span>}
      {offer.oversize && <span className="jm-tag slate">{words.oversize}</span>}
      {offer.ferryTime > 0 && <span className="jm-tag teal">{words.ferry}</span>}
    </>
  )
}

function formatMass(massKg: number, units: Prefs['units'], language: 'en' | 'pt'): string {
  return units === 'mi'
    ? `${Math.round(massKg * 2.204_62).toLocaleString()} lb`
    : `${(massKg / 1000).toLocaleString(language === 'pt' ? 'pt-BR' : 'en-US', { maximumFractionDigits: 1 })} t`
}

function formatVehicleId(raw: string): string {
  const leaf = raw.split('/').pop() ?? raw
  return leaf
    .replace(/\.sii$/i, '')
    .split('_')
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ')
}

function formatTrailerDef(raw: string): string {
  const parts = raw.replace(/^trailer_def\./, '').split('.')
  return parts
    .filter((p) => p && p !== 'scs')
    .map((p) =>
      p
        .split('_')
        .filter(Boolean)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(' '),
    )
    .join(' · ')
}

function emptyMessage(code: JobMarketAnswer, gameRunning: boolean, words: JobMarketWords): string {
  if (!gameRunning && code === 'noAnswer') return words.emptyNoGame
  switch (code) {
    case 'multiplayer':
      return words.emptyMultiplayer
    case 'unsupported':
      return words.emptyUnsupported
    case 'notReady':
      return words.emptyNotReady
    case 'noAnswer':
      return words.emptyNoGame
    default:
      return words.emptyError
  }
}

function formatDuration(minutes: number): string {
  const m = Math.max(1, Math.round(minutes))
  const h = Math.floor(m / 60)
  const rem = m % 60
  if (h >= 24) {
    const d = Math.floor(h / 24)
    const hr = h % 24
    return hr > 0 ? `${d}d ${hr}h` : `${d}d`
  }
  return h > 0 ? `${h}h ${rem}m` : `${rem}m`
}
