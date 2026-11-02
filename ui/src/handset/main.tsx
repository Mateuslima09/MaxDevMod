// First of all: on the player's phone, the way to the PC, in place before the cab's code looks for it.
import { HANDSET_REMOTE } from './install'
import '@fontsource-variable/inter'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Recovery } from '../cabin/shell/Recovery'
import { HandsetApp } from './HandsetApp'
import '../cabin/shell/shell.css'
import '../cabin/shell/apple.css'
import './handset.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Recovery>
      <HandsetApp remote={HANDSET_REMOTE} />
    </Recovery>
  </StrictMode>,
)
