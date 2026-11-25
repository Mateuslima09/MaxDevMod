import {
  firstRequestId,
  isInHost,
  type JobMarketAnswer,
  type JobMarketMode,
  type JobMarketSnapshot,
  postToHost,
} from '../shared/host'

type SnapshotResolver = (res: { answer: JobMarketAnswer; data?: JobMarketSnapshot }) => void
type AnswerResolver = (answer: JobMarketAnswer) => void
type IconResolver = (dataUrl: string | null) => void

let nextId = firstRequestId() + 500_000
const pendingSnapshots = new Map<number, SnapshotResolver>()
const pendingAnswers = new Map<number, AnswerResolver>()
const pendingIcons = new Map<number, IconResolver>()
const iconCache = new Map<string, string | null>()

function allocId(): number {
  nextId += 1
  return nextId
}

export function refreshJobMarket(): Promise<{ answer: JobMarketAnswer; data?: JobMarketSnapshot }> {
  if (!isInHost()) {
    return Promise.resolve({ answer: 'noAnswer' })
  }
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingSnapshots.delete(id)) resolve({ answer: 'noAnswer' })
    }, 6000)
    pendingSnapshots.set(id, (res) => {
      window.clearTimeout(timer)
      resolve(res)
    })
    postToHost({ type: 'jobMarketRefresh', id })
  })
}

export function acceptJobOffer(
  mode: JobMarketMode,
  companyPtr: string,
  offerPtr: string,
): Promise<JobMarketAnswer> {
  if (!isInHost()) {
    return Promise.resolve('noAnswer')
  }
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingAnswers.delete(id)) resolve('noAnswer')
    }, 6000)
    pendingAnswers.set(id, (ans) => {
      window.clearTimeout(timer)
      resolve(ans)
    })
    postToHost({ type: 'jobMarketAccept', id, mode, companyPtr, offerPtr })
  })
}

export function cancelActiveJob(): Promise<JobMarketAnswer> {
  if (!isInHost()) {
    return Promise.resolve('noAnswer')
  }
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingAnswers.delete(id)) resolve('noAnswer')
    }, 6000)
    pendingAnswers.set(id, (ans) => {
      window.clearTimeout(timer)
      resolve(ans)
    })
    postToHost({ type: 'jobMarketCancel', id })
  })
}

export function fetchCargoIcon(icon: string, cargoId: string): Promise<string | null> {
  const key = `${icon}|${cargoId}`
  if (iconCache.has(key)) {
    return Promise.resolve(iconCache.get(key) ?? null)
  }
  if (!isInHost()) {
    return Promise.resolve(null)
  }
  const id = allocId()
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      if (pendingIcons.delete(id)) resolve(null)
    }, 8000)
    pendingIcons.set(id, (dataUrl) => {
      window.clearTimeout(timer)
      iconCache.set(key, dataUrl)
      resolve(dataUrl)
    })
    postToHost({ type: 'jobMarketIcon', id, icon, cargoId })
  })
}

export function receiveJobMarketSnapshot(id: number, answer: JobMarketAnswer, data?: JobMarketSnapshot): void {
  const resolve = pendingSnapshots.get(id)
  if (resolve) {
    pendingSnapshots.delete(id)
    resolve({ answer, data })
  }
}

export function receiveJobMarketAnswer(id: number, answer: JobMarketAnswer): void {
  const resolve = pendingAnswers.get(id)
  if (resolve) {
    pendingAnswers.delete(id)
    resolve(answer)
  }
}

export function receiveJobMarketIcon(id: number, dataUrl: string | null): void {
  const resolve = pendingIcons.get(id)
  if (resolve) {
    pendingIcons.delete(id)
    resolve(dataUrl)
  }
}
