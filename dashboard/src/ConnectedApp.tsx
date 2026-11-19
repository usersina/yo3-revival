import { Input } from './components/form-controls'
import { lazy, Suspense, useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { MutationCache, QueryCache, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { Button, Field, PageHeading, Panel } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  getSignInStaffMutationKey,
  getVerifyStaffCodeMutationKey,
  useGetStaffSession,
  useSignInStaff,
} from './api/generated/http/client'
import { CodeStep, EnrollmentGate, SecurityPanel, StepUpDialog } from './ConnectedMfa'
import { STEP_UP_EVENT, useAcceptSession } from './mfa-session'
import { ApiHttpError, setCsrfToken } from './api/transport'
import { ConnectedShell } from './ConnectedShell'
import { AppearanceSettings, ThemeToggle } from './ThemeControls'
const AccountWorkspace = lazy(() => import('./ConnectedAccounts'))
const ConnectedServer = lazy(() =>
  import('./ConnectedServer').then((m) => ({ default: m.ConnectedServer })),
)
const ConnectedSettings = lazy(() =>
  import('./ConnectedSettings').then((m) => ({ default: m.ConnectedSettings })),
)
const ConnectedInvites = lazy(() =>
  import('./ConnectedInvites').then((m) => ({ default: m.ConnectedInvites })),
)
const ConnectedCodes = lazy(() =>
  import('./ConnectedCodes').then((m) => ({ default: m.ConnectedCodes })),
)
const ConnectedReports = lazy(() =>
  import('./ConnectedReports').then((m) => ({ default: m.ConnectedReports })),
)
const ConnectedAudit = lazy(() =>
  import('./ConnectedAudit').then((m) => ({ default: m.ConnectedAudit })),
)
const ConnectedLogs = lazy(() =>
  import('./ConnectedLogs').then((m) => ({ default: m.ConnectedLogs })),
)
const ConnectedBackups = lazy(() =>
  import('./ConnectedBackups').then((m) => ({ default: m.ConnectedBackups })),
)

function errorText(error: unknown) {
  return error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check that it is running, then try again.'
}

function SignIn() {
  const accept = useAcceptSession()
  const [user, setUser] = useState('')
  const [password, setPassword] = useState('')
  // A right password for an account with an authenticator waits here for its code.
  const [challenge, setChallenge] = useState<string | null>(null)
  const signIn = useSignInStaff({
    mutation: {
      onSuccess: (outcome) => {
        setPassword('')
        if (outcome.code_required && outcome.challenge) setChallenge(outcome.challenge)
        else if (outcome.session) accept(outcome.session, true)
      },
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (signIn.isPending || !user.trim() || !password) return
    signIn.mutate({
      data: { user: user.trim(), password },
      headers: { 'Idempotency-Key': crypto.randomUUID() },
    })
  }
  if (challenge) {
    return (
      <CodeStep
        challenge={challenge}
        onBack={() => {
          setChallenge(null)
          signIn.reset()
        }}
      />
    )
  }
  return (
    <section className="panel auth-card" aria-labelledby="sign-in-heading">
      <div className="auth-heading">
        <h1 id="sign-in-heading">Sign in to your server</h1>
        <p>Use an administrator or moderator game account.</p>
      </div>
      <form onSubmit={submit} className="connected-sign-in">
        <Field id="staff-user" label="User ID">
          <Input
            id="staff-user"
            value={user}
            onChange={(e) => setUser(e.target.value)}
            autoComplete="username"
            required
            maxLength={64}
            disabled={signIn.isPending}
          />
        </Field>
        <Field id="staff-password" label="Password">
          <Input
            id="staff-password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            required
            disabled={signIn.isPending}
          />
        </Field>
        {signIn.error && (
          <p role="alert" className="connected-error">
            {errorText(signIn.error)}
          </p>
        )}
        <Button
          variant="primary"
          type="submit"
          disabled={!user.trim() || !password || signIn.isPending}
        >
          {signIn.isPending && <LoaderCircle size={16} className="animate-spin" />}
          {signIn.isPending ? 'Signing in…' : 'Sign in'}
        </Button>
      </form>
    </section>
  )
}

function Workspace() {
  const { pathname } = useLocation()
  const integration = import.meta.env.DEV && pathname === '/connected'
  const session = useGetStaffSession({ query: { refetchInterval: 60000 } })
  useEffect(() => {
    setCsrfToken(session.data?.csrf_token)
  }, [session.data?.csrf_token])
  const signedOut = session.error instanceof ApiHttpError && session.error.status === 401
  const entry = !session.data || Boolean(session.error) || session.data.limited
  useEffect(() => {
    if (entry) document.title = 'Sign in · YO3 Revival'
  }, [entry])
  if (session.data && !session.error && !session.data.limited && !integration) {
    const user = session.data
    const headings: Record<string, [string, string]> = {
      '/': ['Overview', 'Checked status from your running server.'],
      '/invites': ['Invites & codes', 'Manage registration invites and BOOSTERPASS rewards.'],
      '/news': ['News & maintenance', 'Publish game announcements and plan maintenance.'],
      '/reports': ['Reports', 'Player reports sent with /report in the game’s chat.'],
      '/audit': ['Audit log', 'Persisted administrative changes and saved outcomes.'],
      '/logs': ['Server logs', 'Recent output from this server process.'],
      '/backups': ['Backups', 'Create snapshots and keep a private copy off the host.'],
      '/settings': ['Settings', 'Dashboard appearance and your server’s configuration.'],
    }
    const heading = headings[pathname]
    const accountPath = /^\/accounts\/([^/]+)\/?$/.exec(pathname)
    return (
      <ConnectedShell session={user}>
        <StepUpDialog />
        <Suspense
          fallback={
            <p role="status" className="connected-state">
              Loading workspace…
            </p>
          }
        >
          {pathname === '/accounts' || pathname === '/moderation' || accountPath ? (
            <AccountWorkspace
              key={user.user_id}
              session={user}
              full={false}
              accountId={accountPath?.[1]}
              title={pathname === '/moderation' ? 'Moderation' : 'Accounts'}
            />
          ) : heading ? (
            <>
              <PageHeading title={heading[0]} description={heading[1]} />
              {pathname === '/' && (
                <>
                  <ConnectedServer session={user} />
                  <ConnectedLogs summary />
                </>
              )}
              {pathname === '/invites' && (
                <>
                  <ConnectedInvites session={user} />
                  <ConnectedCodes session={user} />
                </>
              )}
              {pathname === '/news' && <ConnectedSettings key="news" session={user} view="news" />}
              {pathname === '/settings' && (
                <>
                  <AppearanceSettings />
                  <SecurityPanel session={user} />
                  <ConnectedSettings key="registry" session={user} view="registry" />
                </>
              )}
              {pathname === '/reports' && <ConnectedReports session={user} />}
              {pathname === '/audit' && <ConnectedAudit />}
              {pathname === '/logs' && <ConnectedLogs />}
              {pathname === '/backups' && <ConnectedBackups session={user} />}
            </>
          ) : (
            <PageHeading
              title="Page not found"
              description="Choose a page from the navigation to continue."
            />
          )}
        </Suspense>
      </ConnectedShell>
    )
  }
  return (
    <div className="connected-shell">
      <header className="connected-header">
        <Link to="/" className="brand" aria-label="YO3 Revival home">
          <img src="/revival-mark.png" width="43" height="43" alt="" />
          <div>
            <strong>
              YO3 <span>REVIVAL</span>
            </strong>
            <small>SERVER DASHBOARD</small>
          </div>
        </Link>
        <ThemeToggle />
      </header>
      <main id="main" className={`connected-main${entry ? ' auth-main' : ''}`}>
        {integration && (
          <p className="inline-notice">
            Local integration view · Real server data. Use the production build for hosting.
          </p>
        )}
        {session.isPending ? (
          <p role="status" className="panel auth-card connected-state">
            <LoaderCircle size={18} className="spin" />
            Connecting to your server…
          </p>
        ) : signedOut ? (
          <SignIn />
        ) : session.error ? (
          <Panel
            className="auth-card"
            title="Server unavailable"
            description={errorText(session.error)}
          >
            <div className="connected-state">
              <Button onClick={() => void session.refetch()}>Try again</Button>
            </div>
          </Panel>
        ) : session.data?.limited ? (
          <EnrollmentGate session={session.data} />
        ) : session.data ? (
          <AccountWorkspace key={session.data.user_id} session={session.data} />
        ) : null}
      </main>
    </div>
  )
}

export default function ConnectedApp() {
  const [client] = useState(() => {
    function authError(error: unknown) {
      if (error instanceof ApiHttpError && error.status === 401)
        void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
    }
    const client = new QueryClient({
      queryCache: new QueryCache({
        onError: (error, query) => {
          if (query.queryKey[0] !== getGetStaffSessionQueryKey()[0]) authError(error)
        },
      }),
      mutationCache: new MutationCache({
        onError: (error, _variables, _context, mutation) => {
          // A sensitive action wants a recent authenticator check: ask for it.
          if (error instanceof ApiHttpError && error.data.code === 'reauthentication_required')
            window.dispatchEvent(new Event(STEP_UP_EVENT))
          // A refused sign-in or code is a form error, not an expired staff session.
          // Refetching the session would remount the form and discard that error.
          const key = mutation.options.mutationKey?.[0]
          if (key !== getSignInStaffMutationKey()[0] && key !== getVerifyStaffCodeMutationKey()[0])
            authError(error)
        },
      }),
      defaultOptions: { queries: { retry: false }, mutations: { retry: false, gcTime: 0 } },
    })
    return client
  })
  return (
    <QueryClientProvider client={client}>
      <Suspense
        fallback={
          <p role="status" className="connected-state">
            Loading dashboard…
          </p>
        }
      >
        <Workspace />
      </Suspense>
    </QueryClientProvider>
  )
}
