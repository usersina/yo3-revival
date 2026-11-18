import { Component, useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { RefreshCw } from 'lucide-react'
import { Button } from './components/ui'

import { dashboardCommit } from './build'

function ReloadButton() {
  return (
    <Button onClick={() => window.location.reload()}>
      <RefreshCw size={15} />
      Reload dashboard
    </Button>
  )
}

export function DashboardUpdate() {
  const [update, setUpdate] = useState(false)
  const [missing, setMissing] = useState(false)
  useEffect(() => {
    const controller = new AbortController()
    let checking = false
    async function check() {
      if (checking || document.visibilityState === 'hidden') return
      checking = true
      try {
        const response = await fetch(`/assets/dashboard-version.json?check=${Date.now()}`, {
          cache: 'no-store',
          credentials: 'same-origin',
          signal: controller.signal,
        })
        if (!response.ok) return
        const metadata: unknown = await response.json()
        if (
          metadata &&
          typeof metadata === 'object' &&
          'commit' in metadata &&
          typeof metadata.commit === 'string' &&
          /^[0-9a-f]{40}$/.test(metadata.commit) &&
          metadata.commit !== dashboardCommit
        )
          setUpdate(true)
      } catch {
        /* A failed background update check does not interrupt administration. */
      } finally {
        checking = false
      }
    }
    function preloadError(event: Event) {
      event.preventDefault()
      setMissing(true)
    }
    void check()
    const interval = window.setInterval(() => void check(), 60000)
    window.addEventListener('focus', check)
    document.addEventListener('visibilitychange', check)
    window.addEventListener('vite:preloadError', preloadError)
    return () => {
      controller.abort()
      window.clearInterval(interval)
      window.removeEventListener('focus', check)
      document.removeEventListener('visibilitychange', check)
      window.removeEventListener('vite:preloadError', preloadError)
    }
  }, [])
  if (!update && !missing) return null
  return (
    <div role="status" className="dashboard-update">
      <div>
        <strong>
          {missing ? 'This page needs the latest dashboard files.' : 'An update is available.'}
        </strong>
        <span>Save your work before reloading.</span>
      </div>
      <ReloadButton />
    </div>
  )
}

export class PageLoadBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false }
  static getDerivedStateFromError() {
    return { failed: true }
  }
  render() {
    if (this.state.failed)
      return (
        <main className="connected-main">
          <h1>This page could not load</h1>
          <p>Check your connection and reload to get the current dashboard files.</p>
          <ReloadButton />
        </main>
      )
    return this.props.children
  }
}
