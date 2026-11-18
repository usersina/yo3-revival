import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Clipboard, LoaderCircle, RefreshCw, UserPlus } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal } from './components/ui'
import {
  getGetAccountQueryKey,
  getGetStaffSessionQueryKey,
  getListAccountsQueryKey,
  useIssueAccountCode,
} from './api/generated/http/client'
import type { IssueAccountCodeMutationVariables } from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import {
  AccountCodeResponse as ResponseSchema,
  IssueAccountCodeHeader,
  IssueAccountCodeRequest,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

export type CredentialAction =
  { kind: 'create_account' } | { kind: 'reset_password'; user_id: number; login: string }
const intentSchema = z.object({ data: IssueAccountCodeRequest, headers: IssueAccountCodeHeader })
const savedSchema = z.object({
  label: z.string(),
  intent: intentSchema.nullable(),
  result: ResponseSchema.nullable(),
})
type Saved = z.infer<typeof savedSchema>
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Retry the same request.'

// Same 32-character alphabet and OS-backed randomness as the server's human-readable codes. No modulo bias.
function newCode() {
  const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  const code = Array.from(
    crypto.getRandomValues(new Uint8Array(16)),
    (byte) => alphabet[byte & 31],
  ).join('')
  return code.match(/.{4}/g)!.join('-')
}

export function ConnectedCredentials({
  session,
  action,
  onChange,
  showCreate = true,
}: {
  session: StaffSession
  showCreate?: boolean
  action: CredentialAction | null
  onChange: (action: CredentialAction | null) => void
}) {
  const client = useQueryClient()
  const key = `yo3:account-code:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(key)
      return { saved: raw ? savedSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        saved: null,
        error: 'Could not restore a code request. Check the audit log before issuing another code.',
      }
    }
  })
  const [saved, setSaved] = useState<Saved | null>(initial.saved)
  const [storageError, setStorageError] = useState(initial.error)
  const [user, setUser] = useState('')
  const [identityChecked, setIdentityChecked] = useState(false)
  const [checked, setChecked] = useState(false)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const [copyError, setCopyError] = useState('')
  function retain(next: Saved) {
    sessionStorage.setItem(key, JSON.stringify(next))
    setSaved(next)
  }
  const issue = useIssueAccountCode({
    mutation: {
      onSuccess: (result, variables) => {
        const label =
          variables.data.kind === 'create_account'
            ? variables.data.user
            : (saved?.label ??
              (action?.kind === 'reset_password'
                ? action.login
                : `Account #${result.confirmation.user_id}`))
        const next = { label, intent: result.state === 'ready' ? variables : null, result }
        try {
          retain(next)
        } catch {
          setSaved(next)
          setStorageError(
            'Confirmed, but this browser could not retain the result. Copy the code before closing this tab.',
          )
        }
        setChecked(true)
        setFailure('')
        setRefused(false)
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        void client.invalidateQueries({
          queryKey: getGetAccountQueryKey(result.confirmation.user_id),
        })
      },
      onError: (error) => {
        setChecked(false)
        setFailure(errorText(error))
        setRefused(
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status),
        )
        if (error instanceof ApiHttpError && [401, 403].includes(error.status))
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
    },
  })
  useEffect(() => {
    if (!saved?.result || !saved.intent || saved.result.state !== 'ready') return
    const timer = window.setTimeout(
      () => {
        const expired: Saved = {
          label: saved.label,
          intent: null,
          result: { ...saved.result!, state: 'expired' },
        }
        try {
          sessionStorage.setItem(key, JSON.stringify(expired))
          setSaved(expired)
        } catch {
          setSaved(expired)
          setStorageError(
            'The code expired, but browser storage could not be cleared. Close this tab before issuing another.',
          )
        }
      },
      Math.max(0, Date.parse(saved.result.confirmation.expires_at) - Date.now()),
    )
    return () => window.clearTimeout(timer)
  }, [key, saved])

  const create = saved?.intent
    ? saved.intent.data.kind === 'create_account'
    : saved?.result
      ? saved.result.confirmation.kind === 'setup'
      : action?.kind === 'create_account'
  const target =
    saved?.intent?.data.kind === 'create_account'
      ? saved.intent.data.user
      : saved?.result
        ? `Account #${saved.result.confirmation.user_id}`
        : action?.kind === 'reset_password'
          ? `${action.login} · Account #${action.user_id}`
          : user.trim()
  const pending = Boolean(saved?.intent)
  const result = saved?.result
  const ready = checked && result?.state === 'ready' && Boolean(saved?.intent)
  const draftValid = create
    ? /^[A-Za-z0-9]{3,16}$/.test(user.trim()) && session.role === 'admin'
    : identityChecked

  function open() {
    onChange({ kind: 'create_account' })
  }
  function close() {
    if (!issue.isPending) onChange(null)
  }
  function clear() {
    try {
      sessionStorage.removeItem(key)
      setSaved(null)
      setChecked(false)
      setFailure('')
      setStorageError('')
      issue.reset()
      return true
    } catch {
      setStorageError('Could not clear this tab’s code. Close the tab before issuing another.')
      return false
    }
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (issue.isPending || storageError) return
    let intent: IssueAccountCodeMutationVariables | null = saved?.intent ?? null
    if (!intent) {
      if (saved || !action || !draftValid) return
      const code = newCode()
      const data = IssueAccountCodeRequest.parse(
        action.kind === 'create_account'
          ? { kind: action.kind, user: user.trim(), code }
          : { kind: action.kind, user_id: action.user_id, code },
      )
      intent = { data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        retain({
          label: action.kind === 'reset_password' ? action.login : user.trim(),
          intent,
          result: null,
        })
      } catch {
        setStorageError(
          'This browser cannot retain the request. Enable session storage before issuing a code.',
        )
        return
      }
    }
    setFailure('')
    setRefused(false)
    setCopyError('')
    issue.mutate(intent)
  }
  async function copy() {
    if (!ready || !saved?.intent) return
    try {
      await navigator.clipboard.writeText(saved.intent.data.code)
      setCopyError('Copied.')
    } catch {
      setCopyError('Copy failed. Select the code above and copy it manually.')
    }
  }

  return (
    <>
      {(showCreate || saved || storageError) && (
        <Button
          onClick={open}
          disabled={!saved && !storageError && session.role !== 'admin'}
          title={session.role !== 'admin' && !saved ? 'Administrators only' : undefined}
        >
          <UserPlus size={15} />
          {saved ? 'Resume code' : storageError ? 'Review code request' : 'Create account'}
        </Button>
      )}
      <Modal
        open={action !== null}
        onOpenChange={(value) => {
          if (!value) close()
        }}
        title={create ? 'Create account' : 'Reset password'}
        description={
          create
            ? 'The player chooses their own password using a one-time setup code.'
            : 'Verify the player’s identity before giving them access to this account.'
        }
      >
        <form id="account-code-form" className="modal-body connected-profile" onSubmit={submit}>
          {storageError && (
            <p role="alert" className="connected-error">
              {storageError}
            </p>
          )}
          {saved ? (
            <>
              <p>
                <strong>
                  {saved.label} · Account #
                  {result?.confirmation.user_id ??
                    (saved.intent?.data.kind === 'reset_password'
                      ? saved.intent.data.user_id
                      : 'pending')}
                </strong>
              </p>
              {!checked && saved.intent && (
                <p className="inline-notice notice-warning">
                  This tab retained the original request. Confirm it with the server before sharing
                  the code.
                </p>
              )}
              {ready && (
                <>
                  <Field id="account-one-time-code" label="One-time code">
                    <output
                      id="account-one-time-code"
                      aria-label="One-time code"
                      className="account-access-code"
                    >
                      {saved.intent!.data.code}
                    </output>
                  </Field>
                  <p>
                    Give this code to the player privately. In the launcher, choose{' '}
                    <strong>Forgot password</strong>, enter their User ID and this code, then choose
                    a password.
                  </p>
                  <p className="field-hint">
                    Expires {new Date(result!.confirmation.expires_at).toLocaleString()}. Works
                    once. A replacement or role change invalidates it.
                  </p>
                  <p className="field-hint">
                    This tab keeps the code until you press Done or it expires. The server cannot
                    show it again.
                  </p>
                  <Button onClick={() => void copy()}>
                    <Clipboard size={15} />
                    Copy code
                  </Button>
                  {copyError && <p role="status">{copyError}</p>}
                </>
              )}
              {result && result.state !== 'ready' && (
                <p role="status" className="inline-notice notice-warning">
                  {result.state === 'expired'
                    ? 'This code has expired.'
                    : 'This code was used, replaced or invalidated.'}{' '}
                  Retrying did not reactivate it. Issue a new reset code if the player still needs
                  help.
                </p>
              )}
              {checked && result?.state === 'ready' && (
                <Button type="submit" disabled={issue.isPending}>
                  <RefreshCw size={15} />
                  Check code status
                </Button>
              )}
              <p className="field-hint">
                Request ID:{' '}
                {saved.intent?.headers['Idempotency-Key'] ?? result?.confirmation.request_id}
              </p>
            </>
          ) : create ? (
            <Field
              id="account-new-user"
              label="User ID"
              hint="3–16 letters or digits. Case does not matter."
            >
              <input
                id="account-new-user"
                value={user}
                onChange={(event) => setUser(event.target.value)}
                autoComplete="off"
                maxLength={16}
                required
                disabled={issue.isPending || Boolean(storageError)}
              />
            </Field>
          ) : (
            <>
              <p>
                <strong>{target}</strong>
              </p>
              <p>
                Issuing a reset code replaces any previous setup/reset code. Existing sign-ins stay
                active until the player uses it.
              </p>
              <label className="profile-role-confirm">
                <input
                  type="checkbox"
                  checked={identityChecked}
                  onChange={(event) => setIdentityChecked(event.target.checked)}
                  disabled={issue.isPending}
                />
                I have verified this player’s identity.
              </label>
            </>
          )}
          {failure && (
            <div role="alert" className="connected-error">
              <p>
                {refused ? 'The code request was refused.' : 'Could not confirm this code request.'}
              </p>
              <p>{failure}</p>
              {!refused && <p>Retry the same request. It may already have been saved.</p>}
            </div>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">
            {saved ? 'Code retained only in this tab' : 'One use · 24 hours'}
          </span>
          <div className="action-group">
            <Button onClick={close} disabled={issue.isPending}>
              Close
            </Button>
            {refused && (
              <Button
                onClick={() => {
                  if (saved?.intent?.data.kind === 'create_account') setUser(saved.intent.data.user)
                  clear()
                }}
                disabled={issue.isPending}
              >
                Edit request
              </Button>
            )}
            {result && (checked || !saved?.intent) ? (
              <Button
                variant="primary"
                onClick={() => {
                  if (clear()) onChange(null)
                }}
                disabled={issue.isPending}
              >
                Done
              </Button>
            ) : (
              !refused && (
                <Button
                  variant="primary"
                  type="submit"
                  form="account-code-form"
                  disabled={issue.isPending || Boolean(storageError) || (!pending && !draftValid)}
                >
                  {issue.isPending && <LoaderCircle size={16} className="animate-spin" />}
                  {issue.isPending
                    ? 'Confirming…'
                    : pending
                      ? 'Retry same request'
                      : create
                        ? 'Create account'
                        : 'Create reset code'}
                </Button>
              )
            )}
          </div>
        </div>
      </Modal>
    </>
  )
}
