/**
 * The phone pages on the player's home screen (owner 2026-11-03): their web app manifest, served by the host with
 * the key (crates/host remote.rs `manifest`), so the app opens connected, with WestPlay's W (public/icons/app) and
 * a name telling the panel from the phone in the game. The page's words in English, Portuguese and Turkish, as the
 * companion's QR codes are (companion/pages/ScreenPage.tsx PHONE_WORDS).
 */

type Page = 'phone' | 'handset'

const NAMES: Record<'en' | 'pt' | 'tr', Record<Page, { name: string; short: string }>> = {
  en: { phone: { name: 'WestPlay Panel', short: 'Panel' }, handset: { name: 'WestPlay Phone', short: 'Phone' } },
  pt: { phone: { name: 'Painel WestPlay', short: 'Painel' }, handset: { name: 'Celular WestPlay', short: 'Celular' } },
  tr: { phone: { name: 'WestPlay Panel', short: 'Panel' }, handset: { name: 'WestPlay Telefon', short: 'Telefon' } },
}

/** Puts the page's manifest and home-screen name in place, once the key is known. */
export function makeInstallable(key: string) {
  const page: Page = location.pathname.endsWith('/handset.html') ? 'handset' : 'phone'
  const language = navigator.language.startsWith('pt') ? 'pt' : navigator.language.startsWith('tr') ? 'tr' : 'en'
  const { name, short } = NAMES[language][page]
  const query = new URLSearchParams({ page, k: key, name, short })
  const manifest = document.createElement('link')
  manifest.rel = 'manifest'
  manifest.href = `/app.webmanifest?${query.toString().replaceAll('+', '%20')}`
  document.head.append(manifest)
  // An iPhone's home screen takes its name from here, not the manifest.
  const title = document.createElement('meta')
  title.name = 'apple-mobile-web-app-title'
  title.content = short
  document.head.append(title)
}
