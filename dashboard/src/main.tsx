import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import DashboardRoot from './DashboardRoot'
import './styles.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <DashboardRoot />
  </StrictMode>,
)
