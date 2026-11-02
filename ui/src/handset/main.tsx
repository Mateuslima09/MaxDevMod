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
      <HandsetApp />
    </Recovery>
  </StrictMode>,
)
