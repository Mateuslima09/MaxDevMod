import { useState } from 'react'
import { type AppId, AppIcon } from './icons'
import { Segmented } from './ios'
import { type Prefs, setPrefs } from './prefs'

interface TutorialOverlayProps {
  language: 'pt' | 'en'
  prefs: Prefs
  remote: boolean
  ipad: boolean
  onFinish: () => void
}

const TOTAL_STEPS = 7

/**
 * First-time interactive onboarding tutorial for the MaxDevMod in-game phone/iPad.
 * Shown automatically on first use (`!prefs.tutorialDone`) and replayable anytime from Settings.
 */
export function TutorialOverlay({ language, prefs, remote, ipad, onFinish }: TutorialOverlayProps) {
  const [step, setStep] = useState(0)
  const t = COPY[language]
  const isLast = step === TOTAL_STEPS - 1

  const complete = () => {
    setPrefs({ tutorialDone: true })
    onFinish()
  }

  return (
    <div
      className={`ios-tutorial${ipad ? ' is-ipad' : ''}`}
      role="dialog"
      aria-modal="true"
      aria-label={t.ariaLabel}
    >
      <div className="ios-tutorial-top">
        <span className="ios-tutorial-counter">
          {step + 1} / {TOTAL_STEPS}
        </span>
        <div className="ios-tutorial-progress" aria-hidden>
          <div
            className="ios-tutorial-progress-bar"
            style={{ width: `${((step + 1) / TOTAL_STEPS) * 100}%` }}
          />
        </div>
        <button type="button" className="ios-tutorial-skip" onClick={complete}>
          {t.skip}
        </button>
      </div>

      <div className="ios-tutorial-body" key={step}>
        {step === 0 && <WelcomeSlide t={t} />}
        {step === 1 && <ControlsSlide t={t} remote={remote} />}
        {step === 2 && <NavigationSlide t={t} />}
        {step === 3 && (
          <FeatureListSlide
            kicker={t.s4Kicker}
            title={t.s4Title}
            subtitle={t.s4Subtitle}
            items={[
              { app: 'jobMarket', title: t.s4JobTitle, desc: t.s4JobDesc },
              { app: 'garage', title: t.s4GarageTitle, desc: t.s4GarageDesc },
              { app: 'nupay', title: t.s4NupayTitle, desc: t.s4NupayDesc },
            ]}
          />
        )}
        {step === 4 && (
          <FeatureListSlide
            kicker={t.s5Kicker}
            title={t.s5Title}
            subtitle={t.s5Subtitle}
            items={[
              { app: 'commands', title: t.s5CmdTitle, desc: t.s5CmdDesc },
              { app: 'suspension', title: t.s5SuspTitle, desc: t.s5SuspDesc },
              { app: 'brakes', title: t.s5BrakeTitle, desc: t.s5BrakeDesc },
              { app: 'floatScreen', title: t.s5FloatTitle, desc: t.s5FloatDesc },
            ]}
          />
        )}
        {step === 5 && (
          <FeatureListSlide
            kicker={t.s6Kicker}
            title={t.s6Title}
            subtitle={t.s6Subtitle}
            items={[
              { app: 'youtube', title: t.s6YtTitle, desc: t.s6YtDesc },
              { app: 'spotify', title: t.s6MusicTitle, desc: t.s6MusicDesc },
              { app: 'tiktok', title: t.s6TikTokTitle, desc: t.s6TikTokDesc },
              { app: 'character', title: t.s6CharTitle, desc: t.s6CharDesc },
            ]}
          />
        )}
        {step === 6 && <SetupSlide t={t} prefs={prefs} remote={remote} />}
      </div>

      <div className="ios-tutorial-footer">
        <div className="ios-tutorial-dots" role="tablist" aria-label={t.stepsAria}>
          {Array.from({ length: TOTAL_STEPS }, (_, idx) => (
            <button
              key={idx}
              type="button"
              role="tab"
              aria-selected={idx === step}
              aria-label={`${t.stepWord} ${idx + 1}`}
              className={`ios-tutorial-dot${idx === step ? ' on' : ''}`}
              onClick={() => setStep(idx)}
            />
          ))}
        </div>

        <div className="ios-tutorial-actions">
          {step > 0 ? (
            <button
              type="button"
              className="ios-tutorial-btn secondary"
              onClick={() => setStep((s) => Math.max(0, s - 1))}
            >
              {t.back}
            </button>
          ) : (
            <span />
          )}

          {isLast ? (
            <button type="button" className="ios-tutorial-btn primary" onClick={complete}>
              {t.finish}
            </button>
          ) : (
            <button
              type="button"
              className="ios-tutorial-btn primary"
              onClick={() => setStep((s) => Math.min(TOTAL_STEPS - 1, s + 1))}
            >
              {t.next}
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

function WelcomeSlide({ t }: { t: Copy }) {
  return (
    <div className="ios-tutorial-hero">
      <div className="ios-tutorial-badge-ring" aria-hidden>
        <div className="ios-tutorial-icon-cluster">
          <AppIcon id="maps" small />
          <AppIcon id="jobMarket" small />
          <AppIcon id="nupay" small />
          <AppIcon id="garage" small />
        </div>
      </div>
      <span className="ios-tutorial-kicker">{t.s1Kicker}</span>
      <h2 className="ios-tutorial-title">{t.s1Title}</h2>
      <p className="ios-tutorial-desc">{t.s1Desc}</p>
      <div className="ios-tutorial-pills">
        <span>{t.s1Pill1}</span>
        <span>{t.s1Pill2}</span>
        <span>{t.s1Pill3}</span>
      </div>
    </div>
  )
}

function ControlsSlide({ t, remote }: { t: Copy; remote: boolean }) {
  return (
    <div className="ios-tutorial-section">
      <span className="ios-tutorial-kicker">{t.s2Kicker}</span>
      <h2 className="ios-tutorial-title">{t.s2Title}</h2>
      <p className="ios-tutorial-desc">{remote ? t.s2DescRemote : t.s2DescGame}</p>

      <div className="ios-tutorial-cards">
        {!remote ? (
          <>
            <div className="ios-tutorial-key-card">
              <div className="ios-tutorial-keys">
                <kbd>Alt</kbd>
                <span className="ios-tutorial-plus">+</span>
                <kbd>4</kbd>
              </div>
              <div className="ios-tutorial-key-text">
                <strong>{t.s2ToggleTitle}</strong>
                <span>{t.s2ToggleDesc}</span>
              </div>
            </div>

            <div className="ios-tutorial-key-card">
              <div className="ios-tutorial-keys">
                <kbd>Alt</kbd>
                <span className="ios-tutorial-plus">+</span>
                <span className="ios-tutorial-mouse-pill">{t.s2Mouse}</span>
              </div>
              <div className="ios-tutorial-key-text">
                <strong>{t.s2InteractTitle}</strong>
                <span>{t.s2InteractDesc}</span>
              </div>
            </div>

            <div className="ios-tutorial-key-card">
              <div className="ios-tutorial-keys">
                <kbd>F8</kbd>
                <span className="ios-tutorial-plus">/</span>
                <kbd>E</kbd>
              </div>
              <div className="ios-tutorial-key-text">
                <strong>{t.s2CharTitle}</strong>
                <span>{t.s2CharDesc}</span>
              </div>
            </div>
          </>
        ) : (
          <div className="ios-tutorial-key-card">
            <div className="ios-tutorial-keys">
              <span className="ios-tutorial-mouse-pill">Wi-Fi</span>
            </div>
            <div className="ios-tutorial-key-text">
              <strong>{t.s2RemoteCardTitle}</strong>
              <span>{t.s2RemoteCardDesc}</span>
            </div>
          </div>
        )}
      </div>

      <p className="ios-tutorial-tip">{t.s2Tip}</p>
    </div>
  )
}

function NavigationSlide({ t }: { t: Copy }) {
  return (
    <div className="ios-tutorial-section">
      <span className="ios-tutorial-kicker">{t.s3Kicker}</span>
      <h2 className="ios-tutorial-title">{t.s3Title}</h2>
      <p className="ios-tutorial-desc">{t.s3Desc}</p>

      <div className="ios-tutorial-mockup" aria-hidden>
        <div className="ios-tutorial-mockup-pages">
          <div className="ios-tutorial-mockup-page">
            <span className="ios-tutorial-mockup-label">{t.s3Page1}</span>
            <div className="ios-tutorial-mockup-grid">
              <AppIcon id="freight" small />
              <AppIcon id="truck" small />
              <AppIcon id="commands" small />
              <AppIcon id="settings" small />
            </div>
          </div>
          <div className="ios-tutorial-mockup-arrow">‹ ›</div>
          <div className="ios-tutorial-mockup-page">
            <span className="ios-tutorial-mockup-label">{t.s3Page2}</span>
            <div className="ios-tutorial-mockup-grid">
              <AppIcon id="jobMarket" small />
              <AppIcon id="garage" small />
              <AppIcon id="nupay" small />
              <AppIcon id="youtube" small />
            </div>
          </div>
        </div>
        <div className="ios-tutorial-mockup-dock">
          <AppIcon id="maps" small />
          <AppIcon id="music" small />
          <AppIcon id="youtubeMusic" small />
          <AppIcon id="spotify" small />
        </div>
      </div>

      <div className="ios-tutorial-bullets">
        <div>
          <strong>{t.s3SwipeTitle}</strong> — {t.s3SwipeDesc}
        </div>
        <div>
          <strong>{t.s3HomeTitle}</strong> — {t.s3HomeDesc}
        </div>
      </div>
    </div>
  )
}

function FeatureListSlide({
  kicker,
  title,
  subtitle,
  items,
}: {
  kicker: string
  title: string
  subtitle: string
  items: { app: AppId; title: string; desc: string }[]
}) {
  return (
    <div className="ios-tutorial-section">
      <span className="ios-tutorial-kicker">{kicker}</span>
      <h2 className="ios-tutorial-title">{title}</h2>
      <p className="ios-tutorial-desc">{subtitle}</p>
      <div className="ios-tutorial-features">
        {items.map((item) => (
          <div className="ios-tutorial-feature-card" key={item.app}>
            <AppIcon id={item.app} small />
            <div className="ios-tutorial-feature-text">
              <strong>{item.title}</strong>
              <span>{item.desc}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

function SetupSlide({ t, prefs, remote }: { t: Copy; prefs: Prefs; remote: boolean }) {
  return (
    <div className="ios-tutorial-section">
      <span className="ios-tutorial-kicker">{t.s7Kicker}</span>
      <h2 className="ios-tutorial-title">{t.s7Title}</h2>
      <p className="ios-tutorial-desc">{t.s7Desc}</p>

      <div className="ios-tutorial-setup">
        {!remote && (
          <div className="ios-tutorial-setup-row">
            <span className="ios-tutorial-setup-label">{t.s7Device}</span>
            <Segmented
              options={[
                ['phone', 'iPhone'],
                ['ipad', 'iPad'],
              ]}
              chosen={prefs.device}
              choose={(device) => setPrefs({ device: device as Prefs['device'] })}
            />
          </div>
        )}

        <div className="ios-tutorial-setup-row">
          <span className="ios-tutorial-setup-label">{t.s7Units}</span>
          <Segmented
            options={[
              ['km', 'km'],
              ['mi', t.s7Miles],
            ]}
            chosen={prefs.units}
            choose={(units) => setPrefs({ units: units as Prefs['units'] })}
          />
        </div>

        <div className="ios-tutorial-setup-row">
          <span className="ios-tutorial-setup-label">{t.s7Currency}</span>
          <Segmented
            options={[
              ['auto', t.s7GameCurrency],
              ['BRL', 'R$'],
              ['USD', 'US$'],
              ['EUR', '€'],
            ]}
            chosen={prefs.currency}
            choose={(currency) => setPrefs({ currency: currency as Prefs['currency'] })}
          />
        </div>
      </div>

      <p className="ios-tutorial-tip">{t.s7ReplayHint}</p>
    </div>
  )
}

const COPY = {
  pt: {
    ariaLabel: 'Tutorial de primeiro uso do celular MaxDevMod',
    stepsAria: 'Passos do tutorial',
    stepWord: 'Passo',
    skip: 'Pular',
    back: 'Voltar',
    next: 'Próximo',
    finish: 'Começar a usar',

    s1Kicker: 'BEM-VINDO AO MAXDEVMOD',
    s1Title: 'Seu celular dentro do caminhão',
    s1Desc:
      'Controle fretes, garagens, banco, música, suspensão e freios sem sair da cabine — ou use no seu celular real pelo QR Code.',
    s1Pill1: 'iPhone & iPad',
    s1Pill2: 'Ao vivo no jogo',
    s1Pill3: 'QR Code',

    s2Kicker: 'CONTROLES BÁSICOS',
    s2Title: 'Como mexer no celular',
    s2DescGame: 'Enquanto dirige ou a pé, use estas teclas rápidas para abrir e tocar na tela:',
    s2DescRemote: 'No seu celular real, basta tocar e arrastar normalmente — tudo sincroniza em tempo real com o jogo.',
    s2Mouse: 'Mouse',
    s2ToggleTitle: 'Tirar ou guardar o celular',
    s2ToggleDesc: 'Mostra e esconde o celular na tela do jogo a qualquer momento.',
    s2InteractTitle: 'Tocar, rolar e arrastar',
    s2InteractDesc: 'Segure Alt para liberar o cursor do mouse sobre o celular e clicar nos apps.',
    s2CharTitle: 'Modo Personagem e ações',
    s2CharDesc: 'F8 desce/sobe do caminhão; E interage com notas fiscais, bomba e frentista.',
    s2RemoteCardTitle: 'Sincronizado pela rede Wi-Fi',
    s2RemoteCardDesc: 'Cada clique aqui atua direto no ETS2/ATS sem precisar segurar Alt no PC.',
    s2Tip: 'Dica: você pode trocar as teclas em Controles no app MaxDevMod no PC.',

    s3Kicker: 'NAVEGAÇÃO',
    s3Title: 'Páginas, Dock e Voltar',
    s3Desc: 'Os apps ficam divididos em duas páginas e uma barra fixa (Dock) embaixo.',
    s3Page1: 'Página 1',
    s3Page2: 'Página 2',
    s3SwipeTitle: 'Trocar de página',
    s3SwipeDesc: 'arraste para o lado ou clique nas setas ‹ e › perto dos pontinhos.',
    s3HomeTitle: 'Voltar ao Início',
    s3HomeDesc: 'toque em Início no topo ou na barra branca inferior de qualquer app.',

    s4Kicker: 'EMPRESA AO VIVO',
    s4Title: 'Fretes, Garagem e Nupay',
    s4Subtitle: 'Tudo sincronizado diretamente com a economia do jogo, sem reiniciar:',
    s4JobTitle: 'Bolsa de Fretes',
    s4JobDesc: 'Veja fretes, cargas e Trabalho Rápido e aceite ou cancele na hora.',
    s4GarageTitle: 'Garagem',
    s4GarageDesc: 'Troque de caminhão, engate/guarde reboques e melhore garagens ao vivo.',
    s4NupayTitle: 'Nupay',
    s4NupayDesc: 'Acompanhe saldo, extrato de fretes/multas e contrate ou quite empréstimos.',

    s5Kicker: 'CABINE E ESTRADA',
    s5Title: 'Comandos, Suspensão e Freios',
    s5Subtitle: 'Controle funções do caminhão e acompanhe o comportamento na serra:',
    s5CmdTitle: 'Comandos',
    s5CmdDesc: 'Vidros, giroflex, pisca-alerta, eixo, luz interna, buzinas e carreta.',
    s5SuspTitle: 'Suspensão a Ar',
    s5SuspDesc: 'Suba/desça e configure o retorno automático calibrado.',
    s5BrakeTitle: 'Freios',
    s5BrakeDesc: 'Temperatura dos discos em tempo real com fumaça 3D nas rodas.',
    s5FloatTitle: 'Tela Flutuante',
    s5FloatDesc: 'Posicione e ajuste a tela virtual dentro da cabine.',

    s6Kicker: 'ENTRETENIMENTO & IMERSÃO',
    s6Title: 'Mídia, LIVE e Modo Personagem',
    s6Subtitle: 'Música nos alto-falantes do caminhão e imersão completa fora da cabine:',
    s6YtTitle: 'YouTube',
    s6YtDesc: 'Assista vídeos, canais e playlists direto no celular ou iPad.',
    s6MusicTitle: 'Spotify & YouTube Music',
    s6MusicDesc: 'Escolha se o áudio sai nos alto-falantes da cabine ou no dispositivo.',
    s6TikTokTitle: 'TikTok & LIVE',
    s6TikTokDesc: 'Feed oficial e notificações de seguidores e presentes no topo da tela.',
    s6CharTitle: 'Personagem',
    s6CharDesc: 'Gerencie notas fiscais, engate manual e abastecimento com frentista.',

    s7Kicker: 'QUASE LÁ',
    s7Title: 'Deixe do seu jeito',
    s7Desc: 'Escolha suas preferências iniciais agora — você pode mudar tudo depois em Ajustes.',
    s7Device: 'Formato',
    s7Units: 'Distância',
    s7Miles: 'Milhas',
    s7Currency: 'Moeda Nupay',
    s7GameCurrency: 'Jogo',
    s7ReplayHint: 'Você pode rever este tutorial quando quiser em Ajustes → Ver tutorial.',
  },
  en: {
    ariaLabel: 'MaxDevMod phone first-time tutorial',
    stepsAria: 'Tutorial steps',
    stepWord: 'Step',
    skip: 'Skip',
    back: 'Back',
    next: 'Next',
    finish: 'Start using',

    s1Kicker: 'WELCOME TO MAXDEVMOD',
    s1Title: 'Your phone inside the truck',
    s1Desc:
      'Manage jobs, garages, bank loans, music, suspension, and brakes without leaving the cab — or mirror it on your real phone via QR Code.',
    s1Pill1: 'iPhone & iPad',
    s1Pill2: 'Live in-game',
    s1Pill3: 'QR Code',

    s2Kicker: 'ESSENTIAL CONTROLS',
    s2Title: 'How to use the phone',
    s2DescGame: 'While driving or on foot, use these quick keys to open and interact:',
    s2DescRemote: 'On your real phone, just tap and swipe naturally — everything stays synced live with the game.',
    s2Mouse: 'Mouse',
    s2ToggleTitle: 'Take out or put away',
    s2ToggleDesc: 'Shows and hides the phone overlay anytime in the game.',
    s2InteractTitle: 'Tap, scroll, and drag',
    s2InteractDesc: 'Hold Alt to move the mouse over the phone and tap apps or swipe pages.',
    s2CharTitle: 'Character Mode & actions',
    s2CharDesc: 'F8 exits/enters the truck; E interacts with freight papers, pump, and attendant.',
    s2RemoteCardTitle: 'Live Wi-Fi sync',
    s2RemoteCardDesc: 'Every tap works directly in ETS2/ATS without holding Alt on your PC.',
    s2Tip: 'Tip: you can change keybinds under Controls in the MaxDevMod PC app.',

    s3Kicker: 'NAVIGATION',
    s3Title: 'Pages, Dock & Home',
    s3Desc: 'Your apps live across two pages plus a fixed bottom Dock.',
    s3Page1: 'Page 1',
    s3Page2: 'Page 2',
    s3SwipeTitle: 'Switch pages',
    s3SwipeDesc: 'drag sideways or tap the ‹ and › arrows next to the page dots.',
    s3HomeTitle: 'Return Home',
    s3HomeDesc: 'tap Home at the top or the bottom indicator bar inside any app.',

    s4Kicker: 'LIVE COMPANY',
    s4Title: 'Job Market, Garage & Nupay',
    s4Subtitle: 'Synced live with your in-game economy without restarting:',
    s4JobTitle: 'Job Market',
    s4JobDesc: 'Browse Freight, Cargo, and Quick Jobs and accept or cancel immediately.',
    s4GarageTitle: 'Garage',
    s4GarageDesc: 'Switch trucks, attach/store trailers, and upgrade garages live.',
    s4NupayTitle: 'Nupay',
    s4NupayDesc: 'Track real balance, trip ledger, and take or repay bank loans instantly.',

    s5Kicker: 'CAB & ROAD',
    s5Title: 'Commands, Suspension & Brakes',
    s5Subtitle: 'Control truck systems and monitor brake heat on steep descents:',
    s5CmdTitle: 'Commands',
    s5CmdDesc: 'Windows, beacons, hazards, lift axle, cabin light, horns, and trailer.',
    s5SuspTitle: 'Air Suspension',
    s5SuspDesc: 'Raise/lower manually or configure calibrated auto-return timing.',
    s5BrakeTitle: 'Brakes',
    s5BrakeDesc: 'Real-time disc temperature with 3D wheel smoke when overheated.',
    s5FloatTitle: 'Floating Screen',
    s5FloatDesc: 'Position and customize the virtual cab display.',

    s6Kicker: 'MEDIA & IMMERSION',
    s6Title: 'Media, LIVE & Character Mode',
    s6Subtitle: 'Music through truck speakers and full immersion outside the cab:',
    s6YtTitle: 'YouTube',
    s6YtDesc: 'Watch videos, channels, and playlists right on the phone or iPad.',
    s6MusicTitle: 'Spotify & YouTube Music',
    s6MusicDesc: 'Route audio to the truck cab speakers or directly to the device.',
    s6TikTokTitle: 'TikTok & LIVE',
    s6TikTokDesc: 'Official feed plus live follower and gift alerts at the top of the phone.',
    s6CharTitle: 'Character',
    s6CharDesc: 'Toggle freight papers, manual coupling, and attendant refueling.',

    s7Kicker: 'ALMOST READY',
    s7Title: 'Make it yours',
    s7Desc: 'Pick your starting preferences now — you can change them anytime in Settings.',
    s7Device: 'Device',
    s7Units: 'Distance',
    s7Miles: 'Miles',
    s7Currency: 'Nupay currency',
    s7GameCurrency: 'Game',
    s7ReplayHint: 'You can replay this tutorial anytime in Settings → Replay tutorial.',
  },
} as const

type Copy = (typeof COPY)['pt'] | (typeof COPY)['en']
