// First of all: the way to the PC, in place before the cab's code looks for it.
import { PHONE_CONNECTS } from './install'
import '@fontsource-variable/inter'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Recovery } from '../cabin/shell/Recovery'
import { PhoneApp } from './PhoneApp'
import '../cabin/shell/shell.css'
import '../cabin/shell/apple.css'
import './phone.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Recovery>
      <PhoneApp connects={PHONE_CONNECTS} />
    </Recovery>
  </StrictMode>,
)
