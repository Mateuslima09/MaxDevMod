import { useState } from 'react'
import type { Service, SignedIn } from '../accounts'
import type { Note, Words } from '../onboarding/words'
import { Button, Checking, Group, Panel, Row } from '../room/parts'
import { banner, useSignOut, useWords } from '../room/room'
import { GLOW, useScreenAt } from '../room/screen'
import type { AppId } from '../../cabin/state/apps'
import {
  type HostStatus,
  appleMusicSignIn,
  appleMusicSignOut,
  spotifySignIn,
  spotifySignOut,
  tiktokSignIn,
  tiktokSignOut,
  youtubeSignIn,
  youtubeSignOut,
} from '../useHostStatus'
import type { PageProps } from './page'

/** TikTok's colour (the phone's TikTok app has no place on the truck's screen: its Home shows meanwhile). */
const TIKTOK_GLOW = 'oklch(0.63 0.24 15)'

/** A service's words in another's name: each language's own, said of TikTok (its name is the same in all). */
function named<T>(words: T, from: string, to: string): T {
  return JSON.parse(JSON.stringify(words).split(from).join(to)) as T
}

/** Each service: its name, its app on the screen, its sign-in and sign-out, and what its status says. */
const SERVICES: Record<
  Service,
  {
    name: string
    app: AppId | 'home'
    glow: string
    signIn(): Promise<void>
    signOut(): Promise<void>
    signingIn(status: HostStatus | null): boolean
    /** Its words while signing in (YouTube's are the accounts' own). */
    words(words: Words): { finishInWindow: string; signingIn: Note }
  }
> = {
  youtube: {
    name: 'YouTube',
    app: 'youtube',
    glow: GLOW.youtube,
    signIn: youtubeSignIn,
    signOut: youtubeSignOut,
    signingIn: (status) => status?.youtubeSigningIn ?? false,
    words: (words) => words.accounts,
  },
  spotify: {
    name: 'Spotify',
    app: 'spotify',
    glow: GLOW.spotify,
    signIn: spotifySignIn,
    signOut: spotifySignOut,
    signingIn: (status) => status?.spotifySigningIn ?? false,
    words: (words) => words.spotify,
  },
  appleMusic: {
    name: 'Apple Music',
    app: 'music',
    glow: GLOW.music,
    signIn: appleMusicSignIn,
    signOut: appleMusicSignOut,
    signingIn: (status) => status?.appleMusicSigningIn ?? false,
    words: (words) => words.appleMusic,
  },
  tiktok: {
    name: 'TikTok',
    app: 'home',
    glow: TIKTOK_GLOW,
    signIn: tiktokSignIn,
    signOut: tiktokSignOut,
    signingIn: (status) => status?.tiktokSigningIn ?? false,
    words: (words) => named(words.spotify, 'Spotify', 'TikTok'),
  },
}

const ORDER: Service[] = ['youtube', 'spotify', 'appleMusic', 'tiktok']

/**
 * The services WestPlay plays from, each signed in (in its own window, on the desktop) or not. WestPlay only
 * ever knows whether it is. On the room's screen, the app of the service the pointer is on.
 */
export function Accounts({ status, cw, signedIn }: PageProps) {
  const words = useWords()
  const [shown, setShown] = useState<Service>('youtube')
  const signingIn = ORDER.find((service) => SERVICES[service].signingIn(status)) ?? null
  const service = SERVICES[signingIn ?? shown]
  useScreenAt(service.app, service.glow, signingIn ? banner(service.words(words).signingIn, true) : null)
  return (
    <Panel place="page" title={cw.accounts.title} sub={cw.accounts.sub}>
      <Group place={4}>
        {ORDER.map((each) => (
          <Account key={each} service={each} status={status} signedIn={signedIn} hover={() => setShown(each)} />
        ))}
      </Group>
    </Panel>
  )
}

/** A service: signed in or not (as last known; found out as the companion opened if never known), and its
 * button. */
function Account({
  service,
  status,
  signedIn: all,
  hover,
}: {
  service: Service
  status: HostStatus | null
  signedIn: SignedIn
  hover(): void
}) {
  const words = useWords()
  const w = words.accounts
  const s = SERVICES[service]
  const known = all[service]
  const signedIn = known ?? false
  const signingIn = s.signingIn(status)
  const checking = known === null
  const signOut = useSignOut(signedIn, s.signOut)
  return (
    <Row
      title={s.name}
      detail={checking ? undefined : signingIn ? s.words(words).finishInWindow : signedIn ? w.signedIn : w.notSignedIn}
      hover={hover}
      accessory={
        checking ? (
          <Checking />
        ) : signedIn ? (
          <Button glass small busy={signOut.signingOut} onClick={signOut.start}>
            {w.signOut}
          </Button>
        ) : (
          <Button glass small busy={signingIn} onClick={() => s.signIn()}>
            {w.signIn}
          </Button>
        )
      }
    />
  )
}
