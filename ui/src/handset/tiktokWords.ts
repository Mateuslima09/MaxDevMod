/** The phone's TikTok app's own words (TikTokApp.tsx): its tabs, and what it says of the account. */
export interface TikTokWords {
  back: string
  forYou: string
  following: string
  explore: string
  live: string
  profile: string
  loading: string
  /** Not signed in: where to sign in. */
  signedOut: string
  /** TikTok's sign-in window is open on the desktop. */
  signingIn: string
  /** The account's LIVE is on: its follows and gifts come as notifications. */
  onLive: string
  /** On the player's own phone: TikTok is the phone in the game's. */
  remote: string
}

export const TIKTOK_WORDS: Record<'en' | 'pt', TikTokWords> = {
  en: {
    back: 'Back',
    forYou: 'For You',
    following: 'Following',
    explore: 'Explore',
    live: 'LIVE',
    profile: 'Profile',
    loading: 'Opening TikTok…',
    signedOut: 'Sign in under Accounts, in WestPlay',
    signingIn: 'Finish signing in in the TikTok window',
    onLive: 'Your LIVE is on · notifications active',
    remote: 'TikTok opens on the phone in the game. On this device, use the TikTok app itself.',
  },
  pt: {
    back: 'Voltar',
    forYou: 'Para você',
    following: 'Seguindo',
    explore: 'Explorar',
    live: 'LIVE',
    profile: 'Perfil',
    loading: 'Abrindo o TikTok…',
    signedOut: 'Entre na sua conta em Contas, no WestPlay',
    signingIn: 'Termine de entrar na janela do TikTok',
    onLive: 'Sua LIVE está no ar · notificações ativas',
    remote: 'O TikTok abre no celular do jogo. Neste aparelho, use o próprio app do TikTok.',
  },
}
