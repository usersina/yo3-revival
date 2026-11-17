import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { NavLink, useLocation } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import {
  ArrowUpRight,
  DatabaseBackup,
  ExternalLink,
  LayoutDashboard,
  ListChecks,
  LogOut,
  Megaphone,
  Menu,
  Settings2,
  ShieldCheck,
  Terminal,
  Ticket,
  Users,
} from 'lucide-react'
import { Badge, Button, Modal } from './components/ui'
import { useGetServerStatus, useSignOutStaff } from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import { ApiHttpError, setCsrfToken } from './api/transport'
const navigation = [
  { label: 'Overview', path: '/', icon: LayoutDashboard, group: 'Workspace' },
  { label: 'Accounts', path: '/accounts', icon: Users, group: 'Workspace' },
  { label: 'Invites & codes', path: '/invites', icon: Ticket, group: 'Workspace' },
  { label: 'News & maintenance', path: '/news', icon: Megaphone, group: 'Workspace' },
  { label: 'Moderation', path: '/moderation', icon: ShieldCheck, group: 'Manage' },
  { label: 'Audit log', path: '/audit', icon: ListChecks, group: 'Manage' },
  { label: 'Server logs', path: '/logs', icon: Terminal, group: 'Manage' },
  { label: 'Backups', path: '/backups', icon: DatabaseBackup, group: 'Manage' },
  { label: 'Settings', path: '/settings', icon: Settings2, group: 'Manage' },
]
const connectedPages = Object.fromEntries(navigation.map(({ path, label }) => [path, label]))
function Navigation({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Main navigation">
      {['Workspace', 'Manage'].map((group) => (
        <div className="nav-group" key={group}>
          <p className="nav-caption">{group}</p>
          {navigation
            .filter((item) => item.group === group)
            .map(({ label, path, icon: Icon }) => (
              <NavLink
                key={path}
                to={path}
                end={path === '/'}
                onClick={onNavigate}
                className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
              >
                <Icon size={18} strokeWidth={1.8} />
                <span>{label}</span>
              </NavLink>
            ))}
        </div>
      ))}
    </nav>
  )
}
export function ConnectedShell({
  session,
  children,
}: {
  session: StaffSession
  children: ReactNode
}) {
  const client = useQueryClient()
  const status = useGetServerStatus({ query: { refetchInterval: 10000 } })
  const [menu, setMenu] = useState(false)
  const { pathname } = useLocation()
  const current = connectedPages[pathname] ?? 'Page not found'
  const signOut = useSignOutStaff({
    mutation: {
      onSuccess: () => {
        setCsrfToken(undefined)
        void client.resetQueries()
      },
    },
  })
  useEffect(() => {
    document.title = `${current} · YO3 Revival`
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' })
    if (!document.querySelector('[role="dialog"]'))
      document.getElementById('main')?.focus({ preventScroll: true })
  }, [current, pathname])
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <aside className="sidebar">
        <NavLink to="/" aria-label="YO3 Revival home" className="brand">
          <img src="/revival-mark.png" width="43" height="43" alt="" />
          <div>
            <strong>
              YO3 <span>REVIVAL</span>
            </strong>
            <small>SERVER DASHBOARD</small>
          </div>
        </NavLink>
        <div className="workspace-card">
          <div>
            <strong>Your server</strong>
            <span>
              {status.error
                ? 'Connection interrupted'
                : status.data
                  ? `${status.data.online} players online`
                  : 'Checking connection…'}
            </span>
          </div>
          <Badge tone={status.error ? 'amber' : status.data ? 'green' : 'neutral'}>
            {status.error ? 'Offline' : status.data ? 'Connected' : 'Checking'}
          </Badge>
        </div>
        <Navigation />
        <div className="sidebar-bottom">
          <a
            className="nav-link"
            href="https://github.com/usersina/yo3-revival"
            target="_blank"
            rel="noreferrer"
          >
            <ExternalLink size={17} />
            <span>Project on GitHub</span>
            <ArrowUpRight size={14} />
          </a>
          <div className="profile">
            <ShieldCheck size={20} />
            <div>
              <strong>{session.user}</strong>
              <span>{session.role === 'admin' ? 'Administrator' : 'Moderator'}</span>
            </div>
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumbs">
            <Button
              variant="ghost"
              className="mobile-menu icon-button"
              aria-label="Open navigation"
              onClick={() => setMenu(true)}
            >
              <Menu size={20} />
            </Button>
            <span>Server</span>
            <span aria-hidden="true">/</span>
            <strong>{current}</strong>
          </div>
          <Button
            onClick={() => signOut.mutate({ headers: { 'Idempotency-Key': crypto.randomUUID() } })}
            disabled={signOut.isPending}
          >
            <LogOut size={16} />
            {signOut.isPending ? 'Signing out…' : 'Sign out'}
          </Button>
        </header>
        <main id="main" className="main-content" tabIndex={-1}>
          {signOut.error && (
            <p role="alert" className="connected-error">
              {signOut.error instanceof ApiHttpError
                ? signOut.error.message
                : 'Could not sign out. Check the connection and try again.'}
            </p>
          )}
          {children}
          <footer className="page-footer">
            <span>YO3 Revival · Made for one more duel.</span>
            <span>
              {status.data ? `Version ${status.data.version}` : 'Version unavailable'} · Private
              management
            </span>
          </footer>
        </main>
      </div>
      <Modal
        open={menu}
        onOpenChange={setMenu}
        title="Navigation"
        description="Your server workspace."
      >
        <div className="mobile-nav">
          <Navigation onNavigate={() => setMenu(false)} />
        </div>
      </Modal>
    </div>
  )
}
