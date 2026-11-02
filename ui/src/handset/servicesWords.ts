import type { ServicesWords } from './ServicesApp'

/** The Services app's words (ServicesApp.tsx), in English and Portuguese. */
export const SERVICES_WORDS: Record<'en' | 'pt', ServicesWords> = {
  en: {
    call: 'Call a service',
    park: 'Park by itself',
    how: 'How it works',
    howText:
      'Call a service opens the game’s services window (its F7): towing to the nearest service, a garage and more. Pick the option there, on the game’s screen.',
  },
  pt: {
    call: 'Chamar serviço',
    park: 'Estacionar rápido',
    how: 'Como funciona',
    howText:
      'Chamar serviço abre a janela de serviços do jogo (o F7): guincho até o serviço mais próximo, garagem e outros. Escolha a opção lá, na tela do jogo.',
  },
}
