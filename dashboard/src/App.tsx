import { lazy, Suspense, useEffect, useState } from 'react'
import { Link, NavLink, Route, Routes, useLocation } from 'react-router-dom'
import {
  ArrowUpRight,
  ChevronRight,
  DatabaseBackup,
  ExternalLink,
  FlaskConical,
  LayoutDashboard,
  ListChecks,
  Megaphone,
  Menu,
  RotateCcw,
  Settings2,
  ShieldCheck,
  Terminal,
  Ticket,
  Users,
} from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { PreviewProvider } from './store'
import { ThemeToggle } from './ThemeControls'
import { usePreview } from './preview'
import { Avatar, Badge, Button, Modal, Notice } from './components/ui'
import { Overview } from './pages/Overview'
import { Accounts } from './pages/Accounts'
import { Invites } from './pages/Invites'
import { NewsPage, SettingsPage } from './pages/Settings'
import { AuditPage, BackupsPage, LogsPage, ModerationPage } from './pages/Operations'

const navigation: { label: string; path: string; icon: LucideIcon; group: string }[] = [
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

const ConnectedApp = lazy(() => import('./ConnectedApp'))

function Navigation({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Main navigation">
      {['Workspace', 'Manage'].map((group) => (
        <div className="nav-group" key={group}>
          <p className="nav-caption">{group}</p>
          {navigation
            .filter((n) => n.group === group)
            .map(({ label, path, icon: Icon }) => (
              <NavLink
                key={path}
                to={path}
                end={path === '/'}
                aria-label={label}
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

function Shell() {
  const { data, role, setRole, reset, generation } = usePreview()
  const [menu, setMenu] = useState(false)
  const [resetOpen, setResetOpen] = useState(false)
  const { pathname } = useLocation()
  const current = navigation.find((n) => n.path === pathname)?.label ?? 'Not found'
  useEffect(() => {
    document.title = `${current} · YO3 Revival preview`
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' })
    if (!document.querySelector('[role="dialog"]')) {
      document.getElementById('main')?.focus({ preventScroll: true })
    }
  }, [current, pathname])
  const online = data.accounts.filter((a) => a.presence !== 'offline').length

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
          <div className="workspace-mark">
            <span className="status-dot" />
          </div>
          <div>
            <strong>{data.settings.name}</strong>
            <span>{online} players online</span>
          </div>
          <Badge tone="neutral">Beta</Badge>
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
            <Avatar initials={role === 'admin' ? 'TA' : 'TM'} color="green" small />
            <div>
              <strong>{role === 'admin' ? 'Test Admin' : 'Test Moderator'}</strong>
              <span>{role === 'admin' ? 'Administrator' : 'Moderator'}</span>
            </div>
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumbs">
            <Button
              variant="ghost"
              className="icon-button mobile-menu"
              aria-label="Open navigation"
              onClick={() => setMenu(true)}
            >
              <Menu size={20} />
            </Button>
            <span>Workspace</span>
            <ChevronRight size={14} />
            <strong>{current}</strong>
          </div>
          <div className="topbar-right">
            <ThemeToggle />
            <span className="topbar-project">Duel Accelerator, revived.</span>
            <Badge tone="green" dot>
              Sample server
            </Badge>
          </div>
        </header>
        <div className="preview-bar">
          <div>
            <FlaskConical size={15} />
            <span>
              <strong>Design preview.</strong> Sample data · not connected to a server.{' '}
              <Link to="/connected">Connect to a local server</Link>
            </span>
          </div>
          <div className="preview-controls">
            <label className="sr-only" htmlFor="preview-role">
              Preview role
            </label>
            <select
              id="preview-role"
              value={role}
              onChange={(e) => setRole(e.target.value as typeof role)}
            >
              <option value="admin">View as admin</option>
              <option value="moderator">View as moderator</option>
            </select>
            <button className="text-button" onClick={() => setResetOpen(true)}>
              <RotateCcw size={13} />
              <span>Reset preview</span>
            </button>
          </div>
        </div>
        <main id="main" tabIndex={-1} className="main-content">
          <Routes key={generation}>
            <Route path="/" element={<Overview />} />
            <Route path="/accounts" element={<Accounts />} />
            <Route path="/invites" element={<Invites />} />
            <Route path="/news" element={<NewsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/moderation" element={<ModerationPage />} />
            <Route path="/audit" element={<AuditPage />} />
            <Route path="/logs" element={<LogsPage />} />
            <Route path="/backups" element={<BackupsPage />} />
            <Route
              path="*"
              element={
                <div className="empty-state">
                  <h1>Page not found</h1>
                  <p>This address doesn’t match a dashboard page.</p>
                  <NavLink className="button button-primary" to="/">
                    Back to overview
                  </NavLink>
                </div>
              }
            />
          </Routes>
          <footer className="page-footer">
            <span>
              YO3 Revival <span className="footer-dot">·</span> Made for one more duel.
            </span>
            <span>
              UI preview <span className="footer-dot">·</span> Changes reset on reload
            </span>
          </footer>
        </main>
      </div>
      <Notice />
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
      <Modal
        open={resetOpen}
        onOpenChange={setResetOpen}
        title="Reset the preview?"
        description="Restore the original sample accounts, invites and settings. No real server is affected."
      >
        <div className="modal-actions">
          <Button onClick={() => setResetOpen(false)}>Cancel</Button>
          <Button
            variant="primary"
            onClick={() => {
              reset()
              setResetOpen(false)
            }}
          >
            <RotateCcw size={16} />
            Reset preview
          </Button>
        </div>
      </Modal>
    </div>
  )
}

export default function App() {
  const { pathname } = useLocation()
  if (pathname === '/connected')
    return (
      <Suspense fallback={<p role="status">Loading dashboard…</p>}>
        <ConnectedApp />
      </Suspense>
    )
  return (
    <PreviewProvider>
      <Shell />
    </PreviewProvider>
  )
}
