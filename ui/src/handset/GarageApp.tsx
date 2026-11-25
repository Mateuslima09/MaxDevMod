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
import { Segmented } from './ios'
import { distance, formatMoney, type Prefs } from './prefs'

type Tab = 'trucks' | 'trailers' | 'garages'

/**
 * Minha Empresa / My Company (live in-memory Fleet & Garage manager):
 * - Browse all owned trucks, owned trailers, and owned garages directly from live game memory.
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
              const odo = distance(truck.odometerKm, units)
              return (
                <article key={truck.truckPtr} className={`garage-card${truck.isPlayer ? ' active' : ''}`}>
                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <span className="garage-vehicle-icon" aria-hidden>
                        <svg viewBox="0 0 24 24">
                          <path
                            d="M3 7h11v8H3Zm11 2h4l3 3v3h-7ZM7.5 17.5a2 2 0 1 0 0 .01M17.5 17.5a2 2 0 1 0 0 .01"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.8"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                      </span>
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
                    {truck.isPlayer ? (
                      <span className="garage-pill active">{pt ? 'Dirigindo' : 'Driving'}</span>
                    ) : truck.hasAiDriver ? (
                      <span className="garage-pill ai">{pt ? 'Motorista IA' : 'AI Driver'}</span>
                    ) : (
                      <span className="garage-pill idle">{pt ? 'Na garagem' : 'Parked'}</span>
                    )}
                  </div>

                  <div className="garage-meta-row">
                    {truck.plate && <span className="garage-plate">{truck.plate}</span>}
                    <span className="garage-meta-chip">{odo}</span>
                    <span className={`garage-meta-chip${truck.wearPct > 25 ? ' warn' : ''}`}>
                      {pt ? `Conservação ${healthPct}%` : `Condition ${healthPct}%`}
                    </span>
                  </div>

                  <div className="garage-card-footer">
                    {truck.isPlayer ? (
                      <span className="garage-active-note">
                        {pt ? 'Seu caminhão atual em uso' : 'Currently selected truck'}
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
                            ? 'Trocando caminhão...'
                            : 'Switching truck...'
                          : pt
                            ? 'Dirigir este caminhão'
                            : 'Drive this truck'}
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
              const isBusy = busyKey === `trailer-${trailer.trailerPtr}` || (trailer.isAttached && busyKey === 'store-trailer')
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
              return (
                <article key={trailer.trailerPtr} className={`garage-card${trailer.isAttached ? ' active' : ''}`}>
                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <span className="garage-vehicle-icon trailer" aria-hidden>
                        <svg viewBox="0 0 24 24">
                          <path
                            d="M2.5 7h15v8h-15Zm15 6H21v2h-3.5M6.5 17.5a1.8 1.8 0 1 0 0 .01M10.5 17.5a1.8 1.8 0 1 0 0 .01M14.5 17.5a1.8 1.8 0 1 0 0 .01"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.8"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                      </span>
                      <div>
                        <h3 className="garage-card-title">{title}</h3>
                        <p className="garage-card-sub">{subtitle}</p>
                      </div>
                    </div>
                    {trailer.isAttached ? (
                      <span className="garage-pill active">{pt ? 'Em uso' : 'Attached'}</span>
                    ) : (
                      <span className="garage-pill idle">{pt ? 'Guardada' : 'Stored'}</span>
                    )}
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

                  <div className="garage-card-footer">
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
                            ? 'Guardar carreta na garagem'
                            : 'Store trailer in garage'}
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
                            ? 'Engatando carreta...'
                            : 'Attaching trailer...'
                          : pt
                            ? 'Selecionar e usar carreta'
                            : 'Select & attach trailer'}
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

              return (
                <article key={garage.garagePtr} className={`garage-card${garage.tier === 'large' ? ' maxed' : ''}`}>
                  <div className="garage-card-head">
                    <div className="garage-card-title-wrap">
                      <span className="garage-vehicle-icon bay" aria-hidden>
                        <svg viewBox="0 0 24 24">
                          <path
                            d="M3 10 12 4l9 6v10H3Zm4 10v-7h10v7M7 16h10"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.8"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                      </span>
                      <div>
                        <h3 className="garage-card-title">{garage.city}</h3>
                        <p className="garage-card-sub">
                          {tierLabel} · {garage.trailersCount} {pt ? 'carreta(s)' : 'trailer(s)'}
                        </p>
                      </div>
                    </div>
                    <span className={`garage-pill ${garage.tier === 'large' ? 'active' : 'idle'}`}>
                      {prodPct}% {pt ? 'prod.' : 'prod.'}
                    </span>
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

                  <div className="garage-card-footer">
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
                </article>
              )
            })
          )}
        </div>
      )}
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
