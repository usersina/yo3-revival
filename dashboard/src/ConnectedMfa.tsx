import { useEffect, useMemo, useState } from 'react'
import type { FormEvent } from 'react'
import qrcode from 'qrcode-generator'
import { useQueryClient } from '@tanstack/react-query'
import { KeyRound, LoaderCircle, LogOut, RefreshCw, ShieldCheck } from 'lucide-react'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import { Input } from './components/form-controls'
import {
  useConfirmMfaEnrollment,
  useConfirmMfaStepUp,
  useGetStaffSession,
  useRegenerateRecoveryCodes,
  useSignOutEverywhere,
  useSignOutStaff,
  useStartMfaEnrollment,
  useVerifyStaffCode,
} from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import { ApiHttpError, setCsrfToken } from './api/transport'
import { downloadText } from './download'
import { STEP_UP_EVENT, useAcceptSession } from './mfa-session'

function errorText(error: unknown) {
  return error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check that it is running, then try again.'
}

/** The QR code of an `otpauth://` link, drawn as SVG (dark on white, whatever the theme) so no image or script source is needed. */
export function QrCode({ value }: { value: string }) {
  const { size, path } = useMemo(() => {
    const code = qrcode(0, 'M')
    code.addData(value)
    code.make()
    const count = code.getModuleCount()
    let d = ''
    for (let row = 0; row < count; row++)
      for (let col = 0; col < count; col++) if (code.isDark(row, col)) d += `M${col} ${row}h1v1h-1z`
    return { size: count, path: d }
  }, [value])
  return (
    <svg
      className="qr-code"
      role="img"
      aria-label="QR code for your authenticator app"
      viewBox={`-3 -3 ${size + 6} ${size + 6}`}
      shapeRendering="crispEdges"
    >
      <rect x={-3} y={-3} width={size + 6} height={size + 6} fill="#ffffff" />
      <path d={path} fill="#111827" />
    </svg>
  )
}

function groups(secret: string) {
  return secret.replace(/(.{4})/g, '$1 ').trim()
}

/** The second step of signing in: the authenticator's six digits, or a recovery code. */
export function CodeStep({ challenge, onBack }: { challenge: string; onBack: () => void }) {
  const accept = useAcceptSession()
  const [code, setCode] = useState('')
  const [recovery, setRecovery] = useState(false)
  const verify = useVerifyStaffCode({
    mutation: {
      onSuccess: (session) => {
        setCode('')
        accept(session, true)
      },
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (verify.isPending || !code.trim()) return
    verify.mutate({
      data: { challenge, code: code.trim() },
      headers: { 'Idempotency-Key': crypto.randomUUID() },
    })
  }
  return (
    <section className="panel auth-card" aria-labelledby="code-heading">
      <div className="auth-heading">
        <h1 id="code-heading">Enter your code</h1>
        <p>
          {recovery
            ? 'Type one of your recovery codes. Each works once.'
            : 'Open your authenticator app and type the six digits it shows for this server.'}
        </p>
      </div>
      <form onSubmit={submit} className="connected-sign-in">
        <Field
          id="staff-code"
          label={recovery ? 'Recovery code' : 'Authenticator code'}
          hint={recovery ? 'Letters and digits, like ABCD-EFGH-JKLM-NPQR.' : undefined}
        >
          <Input
            id="staff-code"
            value={code}
            onChange={(e) => setCode(e.target.value)}
            autoComplete="one-time-code"
            inputMode={recovery ? 'text' : 'numeric'}
            maxLength={recovery ? 24 : 7}
            autoFocus
            required
            disabled={verify.isPending}
          />
        </Field>
        {verify.error && (
          <p role="alert" className="connected-error">
            {errorText(verify.error)}
          </p>
        )}
        <Button variant="primary" type="submit" disabled={!code.trim() || verify.isPending}>
          {verify.isPending && <LoaderCircle size={16} className="animate-spin" />}
          {verify.isPending ? 'Checking…' : 'Verify and sign in'}
        </Button>
        <div className="auth-links">
          <Button
            variant="ghost"
            onClick={() => {
              setRecovery(!recovery)
              setCode('')
              verify.reset()
            }}
          >
            {recovery ? 'Use my authenticator app' : 'Use a recovery code'}
          </Button>
          <Button variant="ghost" onClick={onBack}>
            Back to password
          </Button>
        </div>
      </form>
    </section>
  )
}

/** Recovery codes, shown once: copy or download them, then confirm they are safe. */
export function RecoveryCodes({
  codes,
  doneLabel,
  onDone,
}: {
  codes: string[]
  doneLabel: string
  onDone: () => void
}) {
  const [saved, setSaved] = useState(false)
  const [copied, setCopied] = useState('')
  const text = codes.join('\n')
  return (
    <div className="mfa-step">
      <p>
        Keep these {codes.length} recovery codes somewhere safe, apart from your phone. Each works
        once if you lose your authenticator. <strong>They are not shown again.</strong>
      </p>
      <ol className="recovery-codes" aria-label="Recovery codes">
        {codes.map((code) => (
          <li key={code}>
            <code>{code}</code>
          </li>
        ))}
      </ol>
      <div className="mfa-actions">
        <Button
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(text)
              setCopied('Copied.')
            } catch {
              setCopied('Copy failed. Select the codes and copy them manually.')
            }
          }}
        >
          Copy codes
        </Button>
        <Button onClick={() => downloadText('yo3-recovery-codes.txt', `${text}\n`)}>
          Download .txt
        </Button>
        {copied && <span role="status">{copied}</span>}
      </div>
      <label className="mfa-confirm">
        <input type="checkbox" checked={saved} onChange={(e) => setSaved(e.target.checked)} />I
        saved these recovery codes
      </label>
      <Button variant="primary" disabled={!saved} onClick={onDone}>
        {doneLabel}
      </Button>
    </div>
  )
}

/** Authenticator setup: the QR code and key, a code to prove it works, then the recovery codes. */
export function Enrollment({
  session,
  replacing = false,
  onFinished,
}: {
  session: StaffSession
  replacing?: boolean
  onFinished: (session: StaffSession) => void
}) {
  const start = useStartMfaEnrollment()
  const [code, setCode] = useState('')
  const [done, setDone] = useState<{ codes: string[]; session: StaffSession } | null>(null)
  const confirm = useConfirmMfaEnrollment({
    mutation: {
      onSuccess: (result) => {
        setCode('')
        setDone({ codes: result.recovery_codes, session: result.session })
      },
    },
  })
  if (done) {
    return (
      <RecoveryCodes
        codes={done.codes}
        doneLabel={replacing ? 'Done' : 'Continue to the dashboard'}
        onDone={() => onFinished(done.session)}
      />
    )
  }
  function begin() {
    start.mutate({ headers: { 'Idempotency-Key': crypto.randomUUID() } })
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (confirm.isPending || !code.trim()) return
    confirm.mutate({
      data: { code: code.trim() },
      headers: { 'Idempotency-Key': crypto.randomUUID() },
    })
  }
  const seed = start.data
  return (
    <div className="mfa-step">
      {!seed ? (
        <>
          <p>
            {replacing
              ? 'This replaces your current authenticator. Codes from the old one stop working once you confirm.'
              : `Signed in as ${session.user}. This server needs a second step: a six-digit code from an authenticator app on your phone, such as Aegis, 2FAS, Google Authenticator or 1Password.`}
          </p>
          {start.error && (
            <p role="alert" className="connected-error">
              {errorText(start.error)}
            </p>
          )}
          <Button variant="primary" onClick={begin} disabled={start.isPending}>
            {start.isPending && <LoaderCircle size={16} className="animate-spin" />}
            {start.isPending ? 'Preparing…' : 'Set up authenticator'}
          </Button>
        </>
      ) : (
        <form onSubmit={submit} className="mfa-step">
          <p>
            Scan this QR code with your authenticator app, or type the key in by hand. Then enter
            the six digits it shows.
          </p>
          <div className="mfa-qr">
            <QrCode value={seed.otpauth_uri} />
            <div>
              <span className="mfa-key-label">Key for {seed.account}</span>
              <code className="mfa-key" data-testid="mfa-secret">
                {groups(seed.secret)}
              </code>
            </div>
          </div>
          <Field id="mfa-enroll-code" label="Code from your app">
            <Input
              id="mfa-enroll-code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              autoComplete="one-time-code"
              inputMode="numeric"
              maxLength={7}
              required
              disabled={confirm.isPending}
            />
          </Field>
          {confirm.error && (
            <p role="alert" className="connected-error">
              {errorText(confirm.error)}
            </p>
          )}
          <Button variant="primary" type="submit" disabled={!code.trim() || confirm.isPending}>
            {confirm.isPending && <LoaderCircle size={16} className="animate-spin" />}
            {confirm.isPending ? 'Checking…' : 'Activate'}
          </Button>
        </form>
      )}
    </div>
  )
}

/** What a session limited to enrollment sees instead of the dashboard (the account has no authenticator yet). */
export function EnrollmentGate({ session }: { session: StaffSession }) {
  const accept = useAcceptSession()
  const client = useQueryClient()
  const signOut = useSignOutStaff({
    mutation: {
      onSuccess: () => {
        setCsrfToken(undefined)
        void client.resetQueries()
      },
    },
  })
  return (
    <section className="panel auth-card" aria-labelledby="enroll-heading">
      <div className="auth-heading">
        <h1 id="enroll-heading">Set up two-step sign-in</h1>
        <p>Every staff account on this server needs an authenticator app before it can continue.</p>
      </div>
      <div className="connected-sign-in">
        <Enrollment session={session} onFinished={(next) => accept(next, true)} />
        {signOut.error && (
          <p role="alert" className="connected-error">
            {errorText(signOut.error)}
          </p>
        )}
        <div className="auth-links">
          <Button
            variant="ghost"
            disabled={signOut.isPending}
            onClick={() => signOut.mutate({ headers: { 'Idempotency-Key': crypto.randomUUID() } })}
          >
            Sign out
          </Button>
        </div>
      </div>
    </section>
  )
}

/** Opens when a sensitive action needs a recent authenticator check. */
export function StepUpDialog() {
  const accept = useAcceptSession()
  const session = useGetStaffSession()
  const [open, setOpen] = useState(false)
  const [confirmed, setConfirmed] = useState(false)
  const [code, setCode] = useState('')
  const check = useConfirmMfaStepUp({
    mutation: {
      onSuccess: (next) => {
        setCode('')
        setConfirmed(true)
        accept(next)
      },
    },
  })
  useEffect(() => {
    const show = () => {
      setConfirmed(false)
      setCode('')
      check.reset()
      setOpen(true)
    }
    window.addEventListener(STEP_UP_EVENT, show)
    return () => window.removeEventListener(STEP_UP_EVENT, show)
  }, [check])
  const enabled = session.data?.mfa_enabled ?? false
  function submit(event: FormEvent) {
    event.preventDefault()
    if (check.isPending || !code.trim()) return
    check.mutate({
      data: { code: code.trim() },
      headers: { 'Idempotency-Key': crypto.randomUUID() },
    })
  }
  return (
    <Modal
      open={open}
      onOpenChange={setOpen}
      title="Confirm it is you"
      description="This action needs your authenticator within the last five minutes."
    >
      {!enabled ? (
        <div className="mfa-step">
          <p>
            Sign out and sign in again, or set up an authenticator under Settings, then repeat the
            action.
          </p>
          <Button onClick={() => setOpen(false)}>Close</Button>
        </div>
      ) : confirmed ? (
        <div className="mfa-step">
          <p role="status">
            Confirmed. Repeat the action: it is allowed for the next five minutes.
          </p>
          <Button variant="primary" onClick={() => setOpen(false)}>
            Close
          </Button>
        </div>
      ) : (
        <form onSubmit={submit} className="mfa-step">
          <Field id="step-up-code" label="Authenticator or recovery code">
            <Input
              id="step-up-code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              autoComplete="one-time-code"
              maxLength={24}
              autoFocus
              required
              disabled={check.isPending}
            />
          </Field>
          {check.error && (
            <p role="alert" className="connected-error">
              {errorText(check.error)}
            </p>
          )}
          <Button variant="primary" type="submit" disabled={!code.trim() || check.isPending}>
            {check.isPending ? 'Checking…' : 'Confirm'}
          </Button>
        </form>
      )}
    </Modal>
  )
}

/** Settings: the state of the second factor, new recovery codes, a new authenticator and sign out everywhere. */
export function SecurityPanel({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const accept = useAcceptSession()
  const [replacing, setReplacing] = useState(false)
  const [codes, setCodes] = useState<string[] | null>(null)
  const regenerate = useRegenerateRecoveryCodes({
    mutation: { onSuccess: (result) => setCodes(result.recovery_codes) },
  })
  const everywhere = useSignOutEverywhere({
    mutation: {
      onSuccess: () => {
        setCsrfToken(undefined)
        void client.resetQueries()
      },
    },
  })
  const recent = session.mfa_recent_until ? new Date(session.mfa_recent_until) : null
  const low = session.mfa_enabled && session.recovery_codes_remaining <= 3
  return (
    <Panel
      title="Two-step sign-in"
      description="An authenticator app adds a six-digit code to your password. Recovery codes are for a lost phone."
      className="security-panel"
    >
      <dl className="profile-facts mfa-facts">
        <div>
          <dt>Authenticator</dt>
          <dd>
            <Badge tone={session.mfa_enabled ? 'green' : 'amber'}>
              {session.mfa_enabled ? 'On' : 'Not set up'}
            </Badge>
          </dd>
        </div>
        <div>
          <dt>Recovery codes</dt>
          <dd>
            {session.mfa_enabled ? (
              <Badge tone={low ? 'amber' : 'neutral'}>
                {session.recovery_codes_remaining} left
              </Badge>
            ) : (
              '—'
            )}
          </dd>
        </div>
        <div>
          <dt>Recent check</dt>
          <dd>
            {recent
              ? `Until ${recent.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
              : 'Not recent: sensitive actions ask for a code'}
          </dd>
        </div>
      </dl>
      {low && (
        <p role="status" className="connected-error">
          You are running low on recovery codes. Make a new set.
        </p>
      )}
      {(regenerate.error || everywhere.error) && (
        <p role="alert" className="connected-error">
          {errorText(regenerate.error ?? everywhere.error)}
        </p>
      )}
      <div className="mfa-actions">
        <Button onClick={() => setReplacing(true)}>
          <KeyRound size={16} />
          {session.mfa_enabled ? 'Replace authenticator' : 'Set up authenticator'}
        </Button>
        {session.mfa_enabled && (
          <Button
            onClick={() =>
              regenerate.mutate({ data: {}, headers: { 'Idempotency-Key': crypto.randomUUID() } })
            }
            disabled={regenerate.isPending}
          >
            <RefreshCw size={16} />
            New recovery codes
          </Button>
        )}
        <Button
          onClick={() =>
            everywhere.mutate({ data: {}, headers: { 'Idempotency-Key': crypto.randomUUID() } })
          }
          disabled={everywhere.isPending}
        >
          <LogOut size={16} />
          Sign out everywhere
        </Button>
      </div>
      <Modal
        open={replacing}
        onOpenChange={setReplacing}
        title={session.mfa_enabled ? 'Replace authenticator' : 'Set up authenticator'}
        description="Scan a new QR code and confirm it with a code."
      >
        {replacing && (
          <Enrollment
            session={session}
            replacing={session.mfa_enabled}
            onFinished={(next) => {
              accept(next)
              setReplacing(false)
            }}
          />
        )}
      </Modal>
      <Modal
        open={codes !== null}
        onOpenChange={(open) => !open && setCodes(null)}
        title="New recovery codes"
        description="The old codes no longer work."
      >
        {codes && <RecoveryCodes codes={codes} doneLabel="Done" onDone={() => setCodes(null)} />}
      </Modal>
      <p className="mfa-footnote">
        <ShieldCheck size={14} aria-hidden="true" /> Lost both your phone and your recovery codes?
        The host removes your authenticator with <code>yo3-server staff-mfa-reset</code>.
      </p>
    </Panel>
  )
}
