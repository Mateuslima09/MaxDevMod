import { useCallback, useEffect, useState } from 'react'
import type { FleetAnswer, FleetGarage, FleetSnapshot, FleetTrailer, FleetTruck } from '../shared/host'
import { useLive } from '../cabin/state/store'
import {
  driveOwnedTruck,
  refreshFleet,
  storeOwnedTrailer,
  upgradeOwnedGarage,
  useOwnedTrailer,
} from './bankFleetBridge'
import { fleetGarageVisual, fleetTrailerVisual, fleetTruckVisual } from './fleetPhotos'
import { Segmented } from './ios'
import { distance, formatMoney, type Prefs } from './prefs'

type Tab = 'trucks' | 'trailers' | 'garages'
type DetailTarget =
  | { kind: 'truck'; ptr: string }
  | { kind: 'trailer'; ptr: string }
  | { kind: 'garage'; ptr: string }

/**
 * Minha Empresa / My Company (live in-memory Fleet & Garage manager):
 * - Browse all owned trucks, owned trailers, and owned garages directly from live game memory,
 *   with themed photos (matching Job Market's visual system) and a full Details view.
 * - Switch to drive any owned truck immediately without restarting.
 * - Attach/use any owned trailer directly behind the truck, or store the current trailer back to garage.
 * - Upgrade owned garages (1 -> 3 slots, 3 -> 5 slots) live in memory with exact in-game pricing.
 */
export function GarageApp({
  language,
  currency,
  units,
}: {
  language: 'en' | 'pt'
  currency: Prefs['currency']
  units: Prefs['units']
}) {
  const wallet = useLive((state) => state.wallet)
  const [tab, setTab] = useState<Tab>('trucks')
  const [snap, setSnap] = useState<FleetSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [busyKey, setBusyKey] = useState<string | null>(null)
  const [confirmUpgrade, setConfirmUpgrade] = useState<string | null>(null)
  const [detail, setDetail] = useState<DetailTarget | null>(null)
  const [toast, setToast] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null)

  const pt = language === 'pt'
  const from = wallet?.currency ?? 'USD'
  const money = (amount: number) => formatMoney(amount, from, currency, language, false)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    const res = await refreshFleet()
    setLoading(false)
    if (res.answer === 'ok' && res.data) {
      setSnap(res.data)
    } else {
      setError(fleetErrorText(res.answer, language))
    }
  }, [language])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    if (!toast) return
    const t = window.setTimeout(() => setToast(null), 4200)
    return () => window.clearTimeout(t)
  }, [toast])

  const liveBalance = wallet?.balance ?? snap?.balance ?? null
  const trucks = snap?.trucks ?? []
  const trailers = snap?.trailers ?? []
  const garages = snap?.garages ?? []
  const totalSlots = garages.reduce((sum, g) => sum + g.slots, 0)
  const usedSlots = garages.reduce((sum, g) => sum + g.usedTrucks, 0)

  const handleDriveTruck = async (truck: FleetTruck) => {
    if (busyKey) return
    setBusyKey(`truck-${truck.truckPtr}`)
    const res = await driveOwnedTruck(truck.truckPtr)
    setBusyKey(null)
    if (res.answer === 'ok') {
      if (res.data) setSnap(res.data)
      setToast({
        kind: 'ok',
        text: pt
          ? `Agora dirigindo ${truck.brand} ${truck.model}!`
          : `Now driving ${truck.brand} ${truck.model}!`,
      })
    } else {
      setToast({ kind: 'err', text: fleetErrorText(res.answer, language) })
    }
  }

  const handleUseTrailer = async (trailer: FleetTrailer) => {
    if (busyKey) return
    setBusyKey(`trailer-${trailer.trailerPtr}`)
    const res = await useOwnedTrailer(trailer.trailerPtr)
    setBusyKey(null)
    if (res.answer === 'ok') {
      if (res.data) setSnap(res.data)
      setToast({
        kind: 'ok',
        text: pt ? 'Carreta engatada no seu caminhão!' : 'Trailer attached to your truck!',
      })
    } else {
      setToast({ kind: 'err', text: fleetErrorText(res.answer, language) })
    }
  }

  const handleStoreTrailer = async () => {
    if (busyKey) return
    setBusyKey('store-trailer')
    const res = await storeOwnedTrailer()
    setBusyKey(null)
    if (res.answer === 'ok') {
      if (res.data) setSnap(res.data)
      setToast({
        kind: 'ok',
        text: pt ? 'Carreta guardada na garagem!' : 'Trailer stored back in garage!',
      })
    } else {
      setToast({ kind: 'err', text: fleetErrorText(res.answer, language) })
    }
  }

  const handleUpgradeGarage = async (garage: FleetGarage) => {
    if (busyKey) return
    setBusyKey(`garage-${garage.garagePtr}`)
    setConfirmUpgrade(null)
    const res = await upgradeOwnedGarage(garage.garagePtr)
    setBusyKey(null)
    if (res.answer === 'ok') {
      if (res.data) setSnap(res.data)
      setToast({
        kind: 'ok',
        text: pt
          ? `Garagem de ${garage.city} expandida para ${garage.nextSlots ?? 5} vagas!`
          : `${garage.city} garage upgraded to ${garage.nextSlots ?? 5} slots!`,
      })
    } else {
      setToast({ kind: 'err', text: fleetErrorText(res.answer, language) })
    }
  }

  // Full Details View for a selected Truck, Trailer, or Garage
  if (detail && snap) {
    if (detail.kind === 'truck') {
      const truck = trucks.find((t) => t.truckPtr === detail.ptr)
      if (truck) {
        const vis = fleetTruckVisual(truck)
        const healthPct = Math.max(0, 100 - truck.wearPct)
        const odo = distance(truck.odometerKm * 1000, units)
        const isBusy = busyKey === `truck-${truck.truckPtr}`
        return (
          <div className="garage-app garage-detail-view">
            <button type="button" className="garage-detail-back" onClick={() => setDetail(null)}>
              <span aria-hidden>‹</span> {pt ? 'Voltar para Caminhões' : 'Back to Trucks'}
            </button>

            <div className="garage-detail-hero">
              <img src={vis.photo} alt={`${truck.brand} ${truck.model}`} />
              <div className="garage-photo-scrim" />
              <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                {vis.badge}
              </span>
              {truck.isPlayer ? (
                <span className="garage-pill active garage-photo-pill">{pt ? 'Dirigindo' : 'Driving'}</span>
              ) : truck.hasAiDriver ? (
                <span className="garage-pill ai garage-photo-pill">{pt ? 'Motorista IA' : 'AI Driver'}</span>
              ) : (
                <span className="garage-pill idle garage-photo-pill">{pt ? 'Na garagem' : 'Parked'}</span>
              )}
              <div className="garage-detail-hero-title">
                <h2>
                  {truck.brand} {truck.model}
                </h2>
                <p>
                  {truck.garageCity} · {pt ? `Vaga #${truck.slotIndex + 1}` : `Bay #${truck.slotIndex + 1}`}
                </p>
              </div>
            </div>

            {toast && (
              <div className={`garage-banner ${toast.kind}`} role="status">
                {toast.text}
              </div>
            )}

            <div className="garage-detail-meter">
              <div className="garage-detail-meter-head">
                <span>{pt ? 'Estado de conservação' : 'Condition'}</span>
                <strong>{healthPct}%</strong>
              </div>
              <div className="garage-detail-bar">
                <div
                  className={`garage-detail-bar-fill${healthPct < 55 ? ' danger' : healthPct < 80 ? ' warn' : ''}`}
                  style={{ width: `${healthPct}%` }}
                />
              </div>
            </div>

            <div className="garage-detail-grid">
              <SpecCard label={pt ? 'Marca' : 'Brand'} value={truck.brand || '—'} />
              <SpecCard label={pt ? 'Modelo' : 'Model'} value={truck.model || '—'} />
              <SpecCard label={pt ? 'Motor' : 'Engine'} value={truck.engine || '—'} />
              <SpecCard label={pt ? 'Chassi' : 'Chassis'} value={truck.chassis || '—'} />
              <SpecCard label={pt ? 'Quilometragem' : 'Odometer'} value={odo} />
              <SpecCard
                label={pt ? 'Placa' : 'License plate'}
                value={
                  truck.plate
                    ? `${truck.plate}${truck.plateCountry ? ` (${truck.plateCountry.toUpperCase()})` : ''}`
                    : '—'
                }
              />
              <SpecCard label={pt ? 'Garagem base' : 'Home garage'} value={truck.garageCity || '—'} />
              <SpecCard
                label={pt ? 'Situação' : 'Status'}
                value={
                  truck.isPlayer
                    ? pt
                      ? 'Em uso por você'
                      : 'Driven by you'
                    : truck.hasAiDriver
                      ? pt
                        ? 'Alocado a motorista IA'
                        : 'Assigned to AI driver'
                      : pt
                        ? 'Disponível na garagem'
                        : 'Available in garage'
                }
              />
            </div>

            <div className="garage-detail-actions">
              {truck.isPlayer ? (
                <span className="garage-active-note">
                  {pt ? 'Este é o caminhão que você está dirigindo agora.' : 'You are currently driving this truck.'}
                </span>
              ) : (
                <button
                  type="button"
                  className="garage-action-btn primary"
                  disabled={Boolean(snap.onJob) || busyKey !== null}
                  onClick={() => void handleDriveTruck(truck)}
                >
                  {isBusy
                    ? pt
                      ? 'Trocando caminhão...'
                      : 'Switching truck...'
                    : pt
                      ? 'Dirigir este caminhão'
                      : 'Drive this truck'}
                </button>
              )}
            </div>
          </div>
        )
      }
    }

    if (detail.kind === 'trailer') {
      const trailer = trailers.find((t) => t.trailerPtr === detail.ptr)
      if (trailer) {
        const vis = fleetTrailerVisual(trailer, language)
        const healthPct = Math.max(0, 100 - trailer.wearPct)
        const title = [trailer.brand, trailer.model].filter(Boolean).join(' ')
        const isBusy =
          busyKey === `trailer-${trailer.trailerPtr}` || (trailer.isAttached && busyKey === 'store-trailer')
        return (
          <div className="garage-app garage-detail-view">
            <button type="button" className="garage-detail-back" onClick={() => setDetail(null)}>
              <span aria-hidden>‹</span> {pt ? 'Voltar para Carretas' : 'Back to Trailers'}
            </button>

            <div className="garage-detail-hero">
              <img src={vis.photo} alt={title} />
              <div className="garage-photo-scrim" />
              <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                {vis.badge}
              </span>
              {trailer.isAttached ? (
                <span className="garage-pill active garage-photo-pill">{pt ? 'Em uso' : 'Attached'}</span>
              ) : (
                <span className="garage-pill idle garage-photo-pill">{pt ? 'Guardada' : 'Stored'}</span>
              )}
              <div className="garage-detail-hero-title">
                <h2>{title || (pt ? 'Carreta própria' : 'Owned trailer')}</h2>
                <p>
                  {trailer.garageCity}
                  {trailer.chainType ? ` · ${trailer.chainType}` : ''}
                </p>
              </div>
            </div>

            {toast && (
              <div className={`garage-banner ${toast.kind}`} role="status">
                {toast.text}
              </div>
            )}

            <div className="garage-detail-meter">
              <div className="garage-detail-meter-head">
                <span>{pt ? 'Estado de conservação' : 'Condition'}</span>
                <strong>{healthPct}%</strong>
              </div>
              <div className="garage-detail-bar">
                <div
                  className={`garage-detail-bar-fill${healthPct < 55 ? ' danger' : healthPct < 80 ? ' warn' : ''}`}
                  style={{ width: `${healthPct}%` }}
                />
              </div>
            </div>

            <div className="garage-detail-grid">
              <SpecCard label={pt ? 'Carroceria' : 'Body'} value={trailer.bodyType || trailer.body || '—'} />
              <SpecCard label={pt ? 'Conjunto' : 'Configuration'} value={trailer.chainType || '—'} />
              <SpecCard
                label={pt ? 'Eixos' : 'Axles'}
                value={trailer.axles > 0 ? `${trailer.axles} ${pt ? 'eixos' : 'axles'}` : '—'}
              />
              <SpecCard
                label={pt ? 'Volume' : 'Volume'}
                value={trailer.volume > 0 ? `${Math.round(trailer.volume)} m³` : '—'}
              />
              <SpecCard
                label={pt ? 'Peso bruto' : 'Gross weight'}
                value={trailer.grossWeight > 0 ? `${(trailer.grossWeight / 1000).toFixed(1)} t` : '—'}
              />
              <SpecCard
                label={pt ? 'Placa' : 'License plate'}
                value={
                  trailer.plate
                    ? `${trailer.plate}${trailer.plateCountry ? ` (${trailer.plateCountry.toUpperCase()})` : ''}`
                    : '—'
                }
              />
              <SpecCard label={pt ? 'Garagem base' : 'Home garage'} value={trailer.garageCity || '—'} />
              <SpecCard
                label={pt ? 'Situação' : 'Status'}
                value={
                  trailer.isAttached
                    ? pt
                      ? 'Engatada no caminhão'
                      : 'Attached to truck'
                    : pt
                      ? 'Guardada na garagem'
                      : 'Stored in garage'
                }
              />
            </div>

            <div className="garage-detail-actions">
              {trailer.isAttached ? (
                <button
                  type="button"
                  className="garage-action-btn secondary"
                  disabled={Boolean(snap.onJob) || busyKey !== null}
                  onClick={() => void handleStoreTrailer()}
                >
                  {isBusy
                    ? pt
                      ? 'Guardando...'
                      : 'Storing...'
                    : pt
                      ? 'Guardar carreta na garagem'
                      : 'Store trailer in garage'}
                </button>
              ) : (
                <button
                  type="button"
                  className="garage-action-btn primary"
                  disabled={Boolean(snap.onJob) || !snap.hasTruck || busyKey !== null}
                  onClick={() => void handleUseTrailer(trailer)}
                >
                  {isBusy
                    ? pt
                      ? 'Engatando carreta...'
                      : 'Attaching trailer...'
                    : pt
                      ? 'Selecionar e usar carreta'
                      : 'Select & attach trailer'}
                </button>
              )}
            </div>
          </div>
        )
      }
    }

    if (detail.kind === 'garage') {
      const garage = garages.find((g) => g.garagePtr === detail.ptr)
      if (garage) {
        const vis = fleetGarageVisual(garage, language)
        const prodPct = Math.round(Math.max(0, Math.min(100, garage.productivity * 100)))
        const affordable = garage.upgradeCost != null && (liveBalance ?? 0) >= garage.upgradeCost
        const isBusy = busyKey === `garage-${garage.garagePtr}`
        const asking = confirmUpgrade === garage.garagePtr
        const garageTrucks = trucks.filter((t) => t.garageCityId === garage.cityId || t.garageCity === garage.city)
        return (
          <div className="garage-app garage-detail-view">
            <button type="button" className="garage-detail-back" onClick={() => setDetail(null)}>
              <span aria-hidden>‹</span> {pt ? 'Voltar para Garagens' : 'Back to Garages'}
            </button>

            <div className="garage-detail-hero">
              <img src={vis.photo} alt={garage.city} />
              <div className="garage-photo-scrim" />
              <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                {vis.badge}
              </span>
              <span className={`garage-pill ${garage.tier === 'large' ? 'active' : 'idle'} garage-photo-pill`}>
                {prodPct}% prod.
              </span>
              <div className="garage-detail-hero-title">
                <h2>{garage.city}</h2>
                <p>
                  {garage.usedTrucks}/{garage.slots} {pt ? 'vagas ocupadas' : 'slots used'} · {garage.trailersCount}{' '}
                  {pt ? 'carreta(s)' : 'trailer(s)'}
                </p>
              </div>
            </div>

            {toast && (
              <div className={`garage-banner ${toast.kind}`} role="status">
                {toast.text}
              </div>
            )}

            <div className="garage-detail-meter">
              <div className="garage-detail-meter-head">
                <span>{pt ? 'Produtividade da garagem' : 'Garage productivity'}</span>
                <strong>{prodPct}%</strong>
              </div>
              <div className="garage-detail-bar">
                <div className="garage-detail-bar-fill" style={{ width: `${prodPct}%` }} />
              </div>
            </div>

            <div className="garage-detail-grid">
              <SpecCard label={pt ? 'Cidade' : 'City'} value={garage.city} />
              <SpecCard label={pt ? 'Capacidade' : 'Capacity'} value={`${garage.slots} ${pt ? 'vagas' : 'slots'}`} />
              <SpecCard label={pt ? 'Caminhões' : 'Trucks'} value={`${garage.usedTrucks}/${garage.slots}`} />
              <SpecCard label={pt ? 'Motoristas' : 'Drivers'} value={`${garage.usedDrivers}/${garage.slots}`} />
              <SpecCard label={pt ? 'Carretas' : 'Trailers'} value={`${garage.trailersCount}`} />
              <SpecCard
                label={pt ? 'Próxima expansão' : 'Next upgrade'}
                value={
                  garage.tier === 'large'
                    ? pt
                      ? 'Máximo (5 vagas)'
                      : 'Maxed (5 slots)'
                    : garage.upgradeCost != null
                      ? `${garage.nextSlots} ${pt ? 'vagas' : 'slots'} · ${money(garage.upgradeCost)}`
                      : '—'
                }
              />
            </div>

            {garageTrucks.length > 0 && (
              <div className="garage-detail-roster">
                <span className="garage-detail-roster-title">
                  {pt ? 'Caminhões nesta garagem' : 'Trucks in this garage'}
                </span>
                {garageTrucks.map((t) => (
                  <button
                    key={t.truckPtr}
                    type="button"
                    className="garage-detail-roster-item"
                    onClick={() => setDetail({ kind: 'truck', ptr: t.truckPtr })}
                  >
                    <div>
                      <strong>
                        {t.brand} {t.model}
                      </strong>
                      <span>
                        {pt ? `Vaga #${t.slotIndex + 1}` : `Bay #${t.slotIndex + 1}`}
                        {t.plate ? ` · ${t.plate}` : ''}
                      </span>
                    </div>
                    <span className="garage-detail-chevron" aria-hidden>
                      ›
                    </span>
                  </button>
                ))}
              </div>
            )}

            <div className="garage-detail-actions">
              {garage.tier === 'large' ? (
                <span className="garage-active-note">
                  {pt ? 'Capacidade máxima atingida (5 vagas)' : 'Maximum capacity reached (5 slots)'}
                </span>
              ) : asking && garage.upgradeCost != null ? (
                <div className="garage-upgrade-confirm">
                  <span>
                    {pt
                      ? `Expandir para ${garage.nextSlots} vagas por ${money(garage.upgradeCost)}?`
                      : `Upgrade to ${garage.nextSlots} slots for ${money(garage.upgradeCost)}?`}
                  </span>
                  <div className="garage-upgrade-btns">
                    <button type="button" onClick={() => setConfirmUpgrade(null)}>
                      {pt ? 'Cancelar' : 'Cancel'}
                    </button>
                    <button
                      type="button"
                      className="primary"
                      disabled={isBusy}
                      onClick={() => void handleUpgradeGarage(garage)}
                    >
                      {isBusy ? '...' : pt ? 'Confirmar' : 'Confirm'}
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  className="garage-action-btn upgrade"
                  disabled={!affordable || busyKey !== null}
                  onClick={() => setConfirmUpgrade(garage.garagePtr)}
                >
                  {isBusy
                    ? pt
                      ? 'Expandindo garagem...'
                      : 'Upgrading garage...'
                    : garage.upgradeCost != null
                      ? affordable
                        ? pt
                          ? `Expandir p/ ${garage.nextSlots} vagas · ${money(garage.upgradeCost)}`
                          : `Upgrade to ${garage.nextSlots} slots · ${money(garage.upgradeCost)}`
                        : pt
                          ? `Saldo insuficiente (${money(garage.upgradeCost)})`
                          : `Insufficient funds (${money(garage.upgradeCost)})`
                      : pt
                        ? 'Expandir garagem'
                        : 'Upgrade garage'}
                </button>
              )}
            </div>
          </div>
        )
      }
    }
  }

  return (
    <div className="garage-app">
      <header className="garage-hero">
        <div className="garage-hero-top">
          <div>
            <span className="garage-kicker">{pt ? 'GESTÃO DE FROTA AO VIVO' : 'LIVE FLEET MANAGEMENT'}</span>
            <h2 className="garage-title">{pt ? 'Minha Empresa' : 'My Company'}</h2>
          </div>
          <button
            type="button"
            className={`garage-refresh-btn${loading ? ' spinning' : ''}`}
            onClick={() => void load()}
            aria-label={pt ? 'Atualizar frota' : 'Refresh fleet'}
          >
            <svg viewBox="0 0 24 24" aria-hidden>
              <path
                d="M20 12a8 8 0 1 1-2.34-5.66M20 4v5h-5"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.1"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>

        <div className="garage-stats-strip">
          <div className="garage-stat">
            <span>{pt ? 'Caminhões' : 'Trucks'}</span>
            <strong>{trucks.length}</strong>
          </div>
          <div className="garage-stat">
            <span>{pt ? 'Carretas' : 'Trailers'}</span>
            <strong>{trailers.length}</strong>
          </div>
          <div className="garage-stat">
            <span>{pt ? 'Vagas' : 'Slots'}</span>
            <strong>
              {usedSlots}/{totalSlots}
            </strong>
          </div>
          <div className="garage-stat money">
            <span>{pt ? 'Saldo' : 'Balance'}</span>
            <strong>{liveBalance == null ? '—' : money(liveBalance)}</strong>
          </div>
        </div>
      </header>

      <div className="garage-tabs">
        <Segmented
          options={[
            ['trucks', `${pt ? 'Caminhões' : 'Trucks'} (${trucks.length})`],
            ['trailers', `${pt ? 'Carretas' : 'Trailers'} (${trailers.length})`],
            ['garages', `${pt ? 'Garagens' : 'Garages'} (${garages.length})`],
          ]}
          chosen={tab}
          choose={(next) => setTab(next as Tab)}
        />
      </div>

      {snap?.onJob && (
        <div className="garage-banner warn" role="status">
          {pt
            ? 'Você está em um frete ativo. Finalize ou cancele o frete para trocar caminhão ou carreta (melhorias de garagem continuam liberadas).'
            : 'You have an active job. Finish or cancel it before switching trucks or trailers (garage upgrades stay available).'}
        </div>
      )}

      {toast && (
        <div className={`garage-banner ${toast.kind}`} role="status">
          {toast.text}
        </div>
      )}

      {error && !snap ? (
        <div className="garage-empty-card">
          <p>{error}</p>
          <button type="button" className="garage-action-btn primary" onClick={() => void load()}>
            {pt ? 'Tentar novamente' : 'Try again'}
          </button>
        </div>
      ) : loading && !snap ? (
        <div className="garage-empty-card">
          <p>{pt ? 'Lendo sua frota e garagens do jogo...' : 'Reading your fleet and garages from the game...'}</p>
        </div>
      ) : tab === 'trucks' ? (
        <div className="garage-list">
          {trucks.length === 0 ? (
            <div className="garage-empty-card">
              <p>{pt ? 'Você ainda não possui caminhões próprios.' : 'You do not own any trucks yet.'}</p>
            </div>
          ) : (
            trucks.map((truck) => {
              const isBusy = busyKey === `truck-${truck.truckPtr}`
              const healthPct = Math.max(0, 100 - truck.wearPct)
              const odo = distance(truck.odometerKm * 1000, units)
              const vis = fleetTruckVisual(truck)
              return (
                <article key={truck.truckPtr} className={`garage-card${truck.isPlayer ? ' active' : ''}`}>
                  <div
                    className="garage-photo-banner"
                    role="button"
                    tabIndex={0}
                    onClick={() => setDetail({ kind: 'truck', ptr: truck.truckPtr })}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') setDetail({ kind: 'truck', ptr: truck.truckPtr })
                    }}
                  >
                    <img src={vis.photo} alt={`${truck.brand} ${truck.model}`} loading="lazy" />
                    <div className="garage-photo-scrim" />
                    <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                      {vis.badge}
                    </span>
                    {truck.isPlayer ? (
                      <span className="garage-pill active garage-photo-pill">{pt ? 'Dirigindo' : 'Driving'}</span>
                    ) : truck.hasAiDriver ? (
                      <span className="garage-pill ai garage-photo-pill">{pt ? 'Motorista IA' : 'AI Driver'}</span>
                    ) : (
                      <span className="garage-pill idle garage-photo-pill">{pt ? 'Na garagem' : 'Parked'}</span>
                    )}
                  </div>

                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <div>
                        <h3 className="garage-card-title">
                          {truck.brand} {truck.model}
                        </h3>
                        <p className="garage-card-sub">
                          {truck.garageCity}
                          {truck.chassis ? ` · ${truck.chassis}` : ''}
                          {truck.engine ? ` · ${truck.engine}` : ''}
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="garage-meta-row">
                    {truck.plate && <span className="garage-plate">{truck.plate}</span>}
                    <span className="garage-meta-chip">{odo}</span>
                    <span className={`garage-meta-chip${truck.wearPct > 25 ? ' warn' : ''}`}>
                      {pt ? `Conservação ${healthPct}%` : `Condition ${healthPct}%`}
                    </span>
                  </div>

                  <div className="garage-card-footer garage-card-footer-split">
                    <button
                      type="button"
                      className="garage-action-btn details"
                      onClick={() => setDetail({ kind: 'truck', ptr: truck.truckPtr })}
                    >
                      {pt ? 'Detalhes' : 'Details'}
                    </button>
                    {truck.isPlayer ? (
                      <span className="garage-active-note">
                        {pt ? 'Caminhão atual' : 'Current truck'}
                      </span>
                    ) : (
                      <button
                        type="button"
                        className="garage-action-btn primary"
                        disabled={Boolean(snap?.onJob) || busyKey !== null}
                        onClick={() => void handleDriveTruck(truck)}
                      >
                        {isBusy
                          ? pt
                            ? 'Trocando...'
                            : 'Switching...'
                          : pt
                            ? 'Dirigir'
                            : 'Drive'}
                      </button>
                    )}
                  </div>
                </article>
              )
            })
          )}
        </div>
      ) : tab === 'trailers' ? (
        <div className="garage-list">
          {trailers.length === 0 ? (
            <div className="garage-empty-card">
              <p>
                {pt
                  ? 'Você ainda não possui carretas próprias nas garagens.'
                  : 'You do not own any trailers in your garages yet.'}
              </p>
            </div>
          ) : (
            trailers.map((trailer) => {
              const isBusy =
                busyKey === `trailer-${trailer.trailerPtr}` || (trailer.isAttached && busyKey === 'store-trailer')
              const healthPct = Math.max(0, 100 - trailer.wearPct)
              const title = [trailer.brand, trailer.model].filter(Boolean).join(' ')
              const subtitle = [
                trailer.garageCity,
                trailer.bodyType || trailer.body,
                trailer.chainType,
                trailer.axles > 0 ? `${trailer.axles} ${pt ? 'eixos' : 'axles'}` : '',
              ]
                .filter(Boolean)
                .join(' · ')
              const vis = fleetTrailerVisual(trailer, language)
              return (
                <article key={trailer.trailerPtr} className={`garage-card${trailer.isAttached ? ' active' : ''}`}>
                  <div
                    className="garage-photo-banner"
                    role="button"
                    tabIndex={0}
                    onClick={() => setDetail({ kind: 'trailer', ptr: trailer.trailerPtr })}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') setDetail({ kind: 'trailer', ptr: trailer.trailerPtr })
                    }}
                  >
                    <img src={vis.photo} alt={title} loading="lazy" />
                    <div className="garage-photo-scrim" />
                    <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                      {vis.badge}
                    </span>
                    {trailer.isAttached ? (
                      <span className="garage-pill active garage-photo-pill">{pt ? 'Em uso' : 'Attached'}</span>
                    ) : (
                      <span className="garage-pill idle garage-photo-pill">{pt ? 'Guardada' : 'Stored'}</span>
                    )}
                  </div>

                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <div>
                        <h3 className="garage-card-title">{title}</h3>
                        <p className="garage-card-sub">{subtitle}</p>
                      </div>
                    </div>
                  </div>

                  <div className="garage-meta-row">
                    {trailer.plate && <span className="garage-plate">{trailer.plate}</span>}
                    {trailer.volume > 0 && <span className="garage-meta-chip">{Math.round(trailer.volume)} m³</span>}
                    {trailer.grossWeight > 0 && (
                      <span className="garage-meta-chip">{(trailer.grossWeight / 1000).toFixed(1)} t</span>
                    )}
                    <span className={`garage-meta-chip${trailer.wearPct > 25 ? ' warn' : ''}`}>
                      {pt ? `Conservação ${healthPct}%` : `Condition ${healthPct}%`}
                    </span>
                  </div>

                  <div className="garage-card-footer garage-card-footer-split">
                    <button
                      type="button"
                      className="garage-action-btn details"
                      onClick={() => setDetail({ kind: 'trailer', ptr: trailer.trailerPtr })}
                    >
                      {pt ? 'Detalhes' : 'Details'}
                    </button>
                    {trailer.isAttached ? (
                      <button
                        type="button"
                        className="garage-action-btn secondary"
                        disabled={Boolean(snap?.onJob) || busyKey !== null}
                        onClick={() => void handleStoreTrailer()}
                      >
                        {isBusy
                          ? pt
                            ? 'Guardando...'
                            : 'Storing...'
                          : pt
                            ? 'Guardar'
                            : 'Store'}
                      </button>
                    ) : (
                      <button
                        type="button"
                        className="garage-action-btn primary"
                        disabled={Boolean(snap?.onJob) || !snap?.hasTruck || busyKey !== null}
                        onClick={() => void handleUseTrailer(trailer)}
                      >
                        {isBusy
                          ? pt
                            ? 'Engatando...'
                            : 'Attaching...'
                          : pt
                            ? 'Usar carreta'
                            : 'Attach'}
                      </button>
                    )}
                  </div>
                </article>
              )
            })
          )}
        </div>
      ) : (
        <div className="garage-list">
          {garages.length === 0 ? (
            <div className="garage-empty-card">
              <p>{pt ? 'Nenhuma garagem encontrada no perfil atual.' : 'No owned garages found.'}</p>
            </div>
          ) : (
            garages.map((garage) => {
              const isBusy = busyKey === `garage-${garage.garagePtr}`
              const asking = confirmUpgrade === garage.garagePtr
              const tierLabel =
                garage.tier === 'large'
                  ? pt
                    ? 'Grande · 5 vagas'
                    : 'Large · 5 slots'
                  : garage.tier === 'small'
                    ? pt
                      ? 'Média · 3 vagas'
                      : 'Small · 3 slots'
                    : pt
                      ? 'Inicial · 1 vaga'
                      : 'Tiny · 1 slot'
              const prodPct = Math.round(Math.max(0, Math.min(100, garage.productivity * 100)))
              const affordable = garage.upgradeCost != null && (liveBalance ?? 0) >= garage.upgradeCost
              const vis = fleetGarageVisual(garage, language)

              return (
                <article key={garage.garagePtr} className={`garage-card${garage.tier === 'large' ? ' maxed' : ''}`}>
                  <div
                    className="garage-photo-banner"
                    role="button"
                    tabIndex={0}
                    onClick={() => setDetail({ kind: 'garage', ptr: garage.garagePtr })}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') setDetail({ kind: 'garage', ptr: garage.garagePtr })
                    }}
                  >
                    <img src={vis.photo} alt={garage.city} loading="lazy" />
                    <div className="garage-photo-scrim" />
                    <span className="garage-photo-badge" style={{ borderColor: vis.accent }}>
                      {vis.badge}
                    </span>
                    <span className={`garage-pill ${garage.tier === 'large' ? 'active' : 'idle'} garage-photo-pill`}>
                      {prodPct}% prod.
                    </span>
                  </div>

                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <div>
                        <h3 className="garage-card-title">{garage.city}</h3>
                        <p className="garage-card-sub">
                          {tierLabel} · {garage.trailersCount} {pt ? 'carreta(s)' : 'trailer(s)'}
                        </p>
                      </div>
                    </div>
                  </div>

                  {/* Visual Bay Slots Grid */}
                  <div className="garage-bays" aria-label={pt ? 'Vagas da garagem' : 'Garage bays'}>
                    {Array.from({ length: garage.slots }).map((_, idx) => {
                      const st = garage.slotStates[idx] ?? 0
                      const cls = st === 2 ? 'player' : st === 1 ? 'occupied' : 'empty'
                      const label =
                        st === 2
                          ? pt
                            ? 'Você'
                            : 'You'
                          : st === 1
                            ? pt
                              ? 'Ocupada'
                              : 'Used'
                            : pt
                              ? 'Livre'
                              : 'Free'
                      return (
                        <div key={idx} className={`garage-bay ${cls}`}>
                          <span className="garage-bay-num">#{idx + 1}</span>
                          <span className="garage-bay-state">{label}</span>
                        </div>
                      )
                    })}
                  </div>

                  <div className="garage-card-footer garage-card-footer-split">
                    <button
                      type="button"
                      className="garage-action-btn details"
                      onClick={() => setDetail({ kind: 'garage', ptr: garage.garagePtr })}
                    >
                      {pt ? 'Detalhes' : 'Details'}
                    </button>
                    {garage.tier === 'large' ? (
                      <span className="garage-active-note">
                        {pt ? 'Máximo (5 vagas)' : 'Max (5 slots)'}
                      </span>
                    ) : asking && garage.upgradeCost != null ? (
                      <div className="garage-upgrade-confirm">
                        <span>
                          {pt
                            ? `Expandir p/ ${garage.nextSlots} vagas (${money(garage.upgradeCost)})?`
                            : `Upgrade to ${garage.nextSlots} slots (${money(garage.upgradeCost)})?`}
                        </span>
                        <div className="garage-upgrade-btns">
                          <button type="button" onClick={() => setConfirmUpgrade(null)}>
                            {pt ? 'Cancelar' : 'Cancel'}
                          </button>
                          <button
                            type="button"
                            className="primary"
                            disabled={isBusy}
                            onClick={() => void handleUpgradeGarage(garage)}
                          >
                            {isBusy ? '...' : pt ? 'Confirmar' : 'Confirm'}
                          </button>
                        </div>
                      </div>
                    ) : (
                      <button
                        type="button"
                        className="garage-action-btn upgrade"
                        disabled={!affordable || busyKey !== null}
                        onClick={() => setConfirmUpgrade(garage.garagePtr)}
                      >
                        {isBusy
                          ? pt
                            ? 'Expandindo...'
                            : 'Upgrading...'
                          : garage.upgradeCost != null
                            ? affordable
                              ? pt
                                ? `Expandir · ${money(garage.upgradeCost)}`
                                : `Upgrade · ${money(garage.upgradeCost)}`
                              : pt
                                ? `Sem saldo (${money(garage.upgradeCost)})`
                                : `Low funds (${money(garage.upgradeCost)})`
                            : pt
                              ? 'Expandir'
                              : 'Upgrade'}
                      </button>
                    )}
                  </div>
                </article>
              )
            })
          )}
        </div>
      )}
    </div>
  )
}

function SpecCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="garage-detail-spec">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  )
}

function fleetErrorText(ans: FleetAnswer, language: 'en' | 'pt'): string {
  const pt = language === 'pt'
  switch (ans) {
    case 'multiplayer':
      return pt ? 'Ações de frota ficam desativadas no TruckersMP.' : 'Fleet actions are disabled in TruckersMP.'
    case 'onJob':
      return pt ? 'Finalize ou cancele o frete atual primeiro.' : 'Finish or cancel your active job first.'
    case 'alreadyActive':
      return pt ? 'Já está selecionado no momento.' : 'Already active right now.'
    case 'insufficientFunds':
      return pt ? 'Saldo insuficiente para expandir esta garagem.' : 'Insufficient balance to upgrade this garage.'
    case 'alreadyMax':
      return pt ? 'Esta garagem já está no tamanho máximo.' : 'This garage is already at maximum size.'
    case 'notFound':
      return pt ? 'Item não encontrado — toque em atualizar.' : 'Item not found — tap refresh.'
    case 'notReady':
    case 'noAnswer':
      return pt ? 'Jogo não conectado. Entre na cabine e tente novamente.' : 'Game not ready. Enter the world and try again.'
    default:
      return pt ? 'Não foi possível concluir a operação.' : 'Could not complete the operation.'
  }
}
