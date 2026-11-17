import { lazy, Suspense, useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useLocation } from 'react-router-dom'
import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
  useQueryClient,
} from '@tanstack/react-query'
import { Layers, LoaderCircle } from 'lucide-react'
import { Button, Field, PageHeading, Panel } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  useGetStaffSession,
  useSignInStaff,
} from './api/generated/http/client'
import { ApiHttpError, setCsrfToken } from './api/transport'
import { ConnectedShell } from './ConnectedShell'
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
  const client = useQueryClient()
  const [user, setUser] = useState('')
  const [password, setPassword] = useState('')
  const signIn = useSignInStaff({
    mutation: {
      onSuccess: (session) => {
        setPassword('')
        setCsrfToken(session.csrf_token)
        client.setQueryData(getGetStaffSessionQueryKey(), session)
        void client.resetQueries({
          predicate: (query) => query.queryKey[0] !== getGetStaffSessionQueryKey()[0],
        })
      },
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (signIn.isPending) return
    signIn.mutate({
      data: { user: user.trim(), password },
      headers: { 'Idempotency-Key': crypto.randomUUID() },
    })
  }
  return (
    <>
      <PageHeading title="Server management" description="Sign in to manage your server." />
      <Panel
        title="Sign in to your server"
        description="Use an administrator or moderator game account."
      >
        <form onSubmit={submit} className="connected-sign-in">
          <Field id="staff-user" label="User ID">
            <input
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
            <input
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
          <Button variant="primary" type="submit" disabled={!user || !password || signIn.isPending}>
            {signIn.isPending && <LoaderCircle size={16} className="animate-spin" />}
            {signIn.isPending ? 'Signing in…' : 'Sign in'}
          </Button>
        </form>
      </Panel>
    </>
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
  if (session.data && !session.error && !integration) {
    const user = session.data
    const headings: Record<string, [string, string]> = {
      '/': ['Overview', 'Checked status from your running server.'],
      '/invites': ['Invites & codes', 'Manage registration invites and BOOSTERPASS rewards.'],
      '/news': ['News & maintenance', 'Publish game announcements and plan maintenance.'],
      '/audit': ['Audit log', 'Persisted administrative changes and saved outcomes.'],
      '/logs': ['Server logs', 'Recent output from this server process.'],
      '/backups': ['Backups', 'Create snapshots and keep a private copy off the host.'],
      '/settings': [
        'Settings',
        'Live configuration, with types and application times supplied by the server.',
      ],
    }
    const heading = headings[pathname]
    return (
      <ConnectedShell session={user}>
        <Suspense
          fallback={
            <p role="status" className="connected-state">
              Loading workspace…
            </p>
          }
        >
          {pathname === '/accounts' || pathname === '/moderation' ? (
            <AccountWorkspace
              key={user.user_id}
              session={user}
              full={false}
              title={pathname === '/moderation' ? 'Moderation' : 'Accounts'}
            />
          ) : heading ? (
            <>
              <PageHeading title={heading[0]} description={heading[1]} />
              {pathname === '/' && <ConnectedServer session={user} />}
              {pathname === '/invites' && (
                <>
                  <ConnectedInvites session={user} />
                  <ConnectedCodes session={user} />
                </>
              )}
              {pathname === '/news' && <ConnectedSettings key="news" session={user} view="news" />}
              {pathname === '/settings' && (
                <ConnectedSettings key="registry" session={user} view="registry" />
              )}
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
        <Link to="/" className="brand">
          <Layers size={23} /> YO3 Revival
        </Link>
        <Link to="/">Dashboard</Link>
      </header>
      <main id="main" className="connected-main">
        {integration && (
          <p className="inline-notice">
            Local integration view · Real server data. Public hosting remains disabled.
          </p>
        )}
        {session.isPending ? (
          <p role="status" className="connected-state">
            Connecting to your server…
          </p>
        ) : signedOut ? (
          <SignIn />
        ) : session.error ? (
          <Panel title="Server unavailable" description={errorText(session.error)}>
            <div className="connected-state">
              <Button onClick={() => void session.refetch()}>Try again</Button>
            </div>
          </Panel>
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
      mutationCache: new MutationCache({ onError: authError }),
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
