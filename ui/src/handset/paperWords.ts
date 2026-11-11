/** The cargo's papers' own words (Paper.tsx): the nota fiscal signed before the trailer is coupled, and the
 * recipient's signature before the cargo is delivered. */
export interface PaperWords {
  title: string
  subtitle: string
  number: string
  issued: string
  shipper: string
  recipient: string
  cargo: string
  weight: string
  distance: string
  freight: string
  vehicle: string
  /** Under the box the driver signs in. */
  driverSigns: string
  signHere: string
  clear: string
  sign: string
  /** The clerk's stamp. */
  stampChecked: string
  stampReleased: string
  collect: string
  collectHint: string
  /** The delivery's: what the recipient declares, the driver's signature kept from the pick-up, theirs. */
  received: string
  driver: string
  recipientSigns: string
  handOver: string
  stampReceived: string
  collectReceipt: string
  collectReceiptHint: string
  close: string
}

export const PAPER_WORDS: Record<'en' | 'pt', PaperWords> = {
  en: {
    title: 'FREIGHT BILL',
    subtitle: 'Road cargo transport document',
    number: 'No.',
    issued: 'Issued',
    shipper: 'Shipper',
    recipient: 'Recipient',
    cargo: 'Cargo',
    weight: 'Weight',
    distance: 'Distance',
    freight: 'Freight',
    vehicle: 'Vehicle',
    driverSigns: 'Driver’s signature',
    signHere: 'Sign here with the mouse',
    clear: 'Clear',
    sign: 'Sign',
    stampChecked: 'CHECKED',
    stampReleased: 'RELEASED',
    collect: 'Collect the bill',
    collectHint: 'Signed and stamped. Collect it to couple the trailer and go.',
    received: 'I received the cargo described above in good condition.',
    driver: 'Driver',
    recipientSigns: 'Recipient’s signature',
    handOver: 'Hand it to the recipient',
    stampReceived: 'RECEIVED',
    collectReceipt: 'Collect the receipt',
    collectReceiptHint: 'Signed by the recipient. Collect it to deliver the cargo.',
    close: 'Put it down',
  },
  pt: {
    title: 'NOTA FISCAL',
    subtitle: 'Documento de transporte rodoviário de carga',
    number: 'Nº',
    issued: 'Emissão',
    shipper: 'Remetente',
    recipient: 'Destinatário',
    cargo: 'Carga',
    weight: 'Peso',
    distance: 'Distância',
    freight: 'Valor do frete',
    vehicle: 'Veículo',
    driverSigns: 'Assinatura do motorista',
    signHere: 'Assine aqui com o mouse',
    clear: 'Limpar',
    sign: 'Assinar',
    stampChecked: 'CONFERIDO',
    stampReleased: 'LIBERADO',
    collect: 'Coletar a nota fiscal',
    collectHint: 'Assinada e carimbada. Colete para engatar a carreta e viajar.',
    received: 'Recebi a carga acima descrita em perfeito estado.',
    driver: 'Motorista',
    recipientSigns: 'Assinatura do destinatário',
    handOver: 'Entregar ao destinatário',
    stampReceived: 'RECEBIDO',
    collectReceipt: 'Coletar o canhoto',
    collectReceiptHint: 'Assinado pelo destinatário. Colete para entregar a carga.',
    close: 'Largar a nota',
  },
}

/** A paper's number, the same for the same job: six digits from what the job is. */
export function paperNumber(job: { cargo: string; sourceCity: string; destinationCity: string; deliveryTime: number }) {
  const text = `${job.cargo}|${job.sourceCity}|${job.destinationCity}|${job.deliveryTime}`
  let hash = 2166136261
  for (let i = 0; i < text.length; i++) hash = Math.imul(hash ^ text.charCodeAt(i), 16777619)
  return String((hash >>> 0) % 1_000_000).padStart(6, '0')
}

/** Kilograms as tonnes, as the phone's language writes them ("18,5 t"). */
export function tonnes(kilograms: number, language: 'en' | 'pt'): string {
  const locale = language === 'pt' ? 'pt-BR' : 'en-US'
  return `${(kilograms / 1000).toLocaleString(locale, { maximumFractionDigits: 1 })} t`
}

/** How much ink a signature needs before it counts (px drawn): a dot or a slip of the mouse doesn't. */
export const INK_ENOUGH = 90

/** Whether what was drawn is a signature: enough of it. */
export function isSigned(inkDrawn: number): boolean {
  return inkDrawn >= INK_ENOUGH
}
