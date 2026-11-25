import {
  type BankAnswer,
  type FleetAnswer,
  type FleetSnapshot,
  firstRequestId,
  isInHost,
  postToHost,
} from '../shared/host'

type BankResolver = (answer: BankAnswer) => void
type FleetSnapshotResolver = (res: { answer: FleetAnswer; data?: FleetSnapshot }) => void
type FleetActionResolver = (res: { answer: FleetAnswer; data?: FleetSnapshot }) => void

let nextId = firstRequestId() + 700_000
const pendingBank = new Map<number, BankResolver>()
const pendingFleetSnapshots = new Map<number, FleetSnapshotResolver>()
const pendingFleetActions = new Map<number, FleetActionResolver>()

function allocId(): number {
  nextId += 1
  return nextId
}

export function refreshLiveBank(): Promise<BankAnswer> {
  if (!isInHost()) return Promise.resolve('noAnswer')
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingBank.delete(id)) resolve('noAnswer')
    }, 6000)
    pendingBank.set(id, (ans) => {
      window.clearTimeout(timer)
      resolve(ans)
    })
    postToHost({ type: 'bankRefresh', id })
  })
}

export function takeLiveLoan(offerIndex: number): Promise<BankAnswer> {
  if (!isInHost()) return Promise.resolve('noAnswer')
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingBank.delete(id)) resolve('noAnswer')
    }, 6000)
    pendingBank.set(id, (ans) => {
      window.clearTimeout(timer)
      resolve(ans)
    })
    postToHost({ type: 'bankTakeLoan', id, offerIndex })
  })
}

export function repayLiveLoan(loanIndex: number): Promise<BankAnswer> {
  if (!isInHost()) return Promise.resolve('noAnswer')
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingBank.delete(id)) resolve('noAnswer')
    }, 6000)
    pendingBank.set(id, (ans) => {
      window.clearTimeout(timer)
      resolve(ans)
    })
    postToHost({ type: 'bankRepayLoan', id, loanIndex })
  })
}

export function refreshFleet(): Promise<{ answer: FleetAnswer; data?: FleetSnapshot }> {
  if (!isInHost()) return Promise.resolve({ answer: 'noAnswer' })
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingFleetSnapshots.delete(id)) resolve({ answer: 'noAnswer' })
    }, 7000)
    pendingFleetSnapshots.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'fleetRefresh', id })
  })
}

export function driveOwnedTruck(truckPtr: string): Promise<{ answer: FleetAnswer; data?: FleetSnapshot }> {
  if (!isInHost()) return Promise.resolve({ answer: 'noAnswer' })
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingFleetActions.delete(id)) resolve({ answer: 'noAnswer' })
    }, 7000)
    pendingFleetActions.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'fleetDriveTruck', id, truckPtr })
  })
}

export function useOwnedTrailer(trailerPtr: string): Promise<{ answer: FleetAnswer; data?: FleetSnapshot }> {
  if (!isInHost()) return Promise.resolve({ answer: 'noAnswer' })
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingFleetActions.delete(id)) resolve({ answer: 'noAnswer' })
    }, 7000)
    pendingFleetActions.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'fleetUseTrailer', id, trailerPtr })
  })
}

export function storeOwnedTrailer(): Promise<{ answer: FleetAnswer; data?: FleetSnapshot }> {
  if (!isInHost()) return Promise.resolve({ answer: 'noAnswer' })
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingFleetActions.delete(id)) resolve({ answer: 'noAnswer' })
    }, 7000)
    pendingFleetActions.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'fleetStoreTrailer', id })
  })
}

export function upgradeOwnedGarage(garagePtr: string): Promise<{ answer: FleetAnswer; data?: FleetSnapshot }> {
  if (!isInHost()) return Promise.resolve({ answer: 'noAnswer' })
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingFleetActions.delete(id)) resolve({ answer: 'noAnswer' })
    }, 7000)
    pendingFleetActions.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'fleetUpgradeGarage', id, garagePtr })
  })
}

export function receiveBankAnswer(id: number, answer: BankAnswer): void {
  const resolve = pendingBank.get(id)
  if (resolve) {
    pendingBank.delete(id)
    resolve(answer)
  }
}

export function receiveFleetSnapshot(id: number, answer: FleetAnswer, data?: FleetSnapshot): void {
  const resolve = pendingFleetSnapshots.get(id)
  if (resolve) {
    pendingFleetSnapshots.delete(id)
    resolve({ answer, data })
  }
}

export function receiveFleetAnswer(id: number, answer: FleetAnswer, data?: FleetSnapshot): void {
  const resolve = pendingFleetActions.get(id)
  if (resolve) {
    pendingFleetActions.delete(id)
    resolve({ answer, data })
  }
}
