/** A fine's offence (the SDK's `fine.offence`) as Nupay and the phone's notifications say it. */
export const OFFENCES = {
  en: {
    crash: 'Crash',
    avoid_sleeping: 'Driving tired',
    wrong_way: 'Wrong way',
    speeding_camera: 'Speed camera',
    no_lights: 'No lights',
    red_signal: 'Red light',
    speeding: 'Speeding',
    avoid_weighing: 'Skipped the weigh station',
    illegal_trailer: 'Illegal trailer',
    avoid_inspection: 'Skipped the inspection',
    illegal_border_crossing: 'Illegal border crossing',
    hard_shoulder_violation: 'Hard shoulder',
    damaged_vehicle_usage: 'Damaged vehicle',
    generic: 'Offence',
  },
  pt: {
    crash: 'Batida',
    avoid_sleeping: 'Dirigir com sono',
    wrong_way: 'Contramão',
    speeding_camera: 'Radar',
    no_lights: 'Sem faróis',
    red_signal: 'Sinal vermelho',
    speeding: 'Excesso de velocidade',
    avoid_weighing: 'Fugiu da balança',
    illegal_trailer: 'Carreta irregular',
    avoid_inspection: 'Fugiu da inspeção',
    illegal_border_crossing: 'Fronteira ilegal',
    hard_shoulder_violation: 'Acostamento',
    damaged_vehicle_usage: 'Veículo danificado',
    generic: 'Infração',
  },
}

export function offenceName(offence: string, language: 'en' | 'pt'): string {
  return (OFFENCES[language] as Record<string, string | undefined>)[offence] ?? offence
}
