import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import DashboardRoot from './DashboardRoot'
import './styles.css'
import './preferences.css'
import { initializeTheme } from './theme'

initializeTheme()
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <DashboardRoot />
  </StrictMode>,
)
