/** The Commands app's words (CommandsApp.tsx), in English and Portuguese. */
export interface CommandWords {
  windows: string
  leftDown: string
  leftUp: string
  rightDown: string
  rightUp: string
  suspension: string
  frontUp: string
  frontDown: string
  rearUp: string
  rearDown: string
  reset: string
  truck: string
  beacon: string
  hazard: string
  liftAxle: string
  cabinLight: string
  horn: string
  airHorn: string
  wipers: string
  parkingBrake: string
  attach: string
}

export const COMMAND_WORDS: Record<'en' | 'pt', CommandWords> = {
  en: {
    windows: 'Windows',
    leftDown: 'Driver down',
    leftUp: 'Driver up',
    rightDown: 'Passenger down',
    rightUp: 'Passenger up',
    suspension: 'Air suspension',
    frontUp: 'Front up',
    frontDown: 'Front down',
    rearUp: 'Rear up',
    rearDown: 'Rear down',
    reset: 'Back to normal',
    truck: 'Truck',
    beacon: 'Beacon',
    hazard: 'Hazards',
    liftAxle: 'Lift axle',
    cabinLight: 'Cabin light',
    horn: 'Horn',
    airHorn: 'Air horn',
    wipers: 'Wipers',
    parkingBrake: 'Parking brake',
    attach: 'Hook up / drop trailer',
  },
  pt: {
    windows: 'Vidros',
    leftDown: 'Motorista abrir',
    leftUp: 'Motorista fechar',
    rightDown: 'Passageiro abrir',
    rightUp: 'Passageiro fechar',
    suspension: 'Suspensão a ar',
    frontUp: 'Frente subir',
    frontDown: 'Frente descer',
    rearUp: 'Traseira subir',
    rearDown: 'Traseira descer',
    reset: 'Voltar ao normal',
    truck: 'Caminhão',
    beacon: 'Giroflex',
    hazard: 'Pisca-alerta',
    liftAxle: 'Eixo levantável',
    cabinLight: 'Luz da cabine',
    horn: 'Buzina',
    airHorn: 'Buzina a ar',
    wipers: 'Limpador',
    parkingBrake: 'Freio de mão',
    attach: 'Engatar / soltar carreta',
  },
}
