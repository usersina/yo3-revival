import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Clipboard, KeyRound, LoaderCircle } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal } from './components/ui'
import {
  getListAccountsQueryKey,
  getGetAccountQueryKey,
  getGetStaffSessionQueryKey,
  useManageAccountKey,
} from './api/generated/http/client'
import type { ManageAccountKeyMutationVariables } from './api/generated/http/client'
import type { KeyAccountResult, StaffSession } from './api/generated/http/models'
import { ManageAccountKeyHeader, KeyAccountRequest } from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

export type KeyAction = { kind: 'create' } | { kind: 'reveal'; user_id: number }
const pendingSchema = z.object({ data: KeyAccountRequest, headers: ManageAccountKeyHeader })
const errorText = (e: unknown) =>
  e instanceof ApiHttpError ? e.message : 'Could not reach the server. Retry the same request.'

export function ConnectedKeys({
  session,
  action,
  onChange,
}: {
  session: StaffSession
  action: KeyAction | null
  onChange: (value: KeyAction | null) => void
}) {
  const client = useQueryClient()
  const storageKey = `yo3:key-request:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error:
          'Could not restore a key request. Check the audit log before creating another account.',
      }
    }
  })
  const [pending, setPending] = useState<ManageAccountKeyMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [names, setNames] = useState('')
  const [accepted, setAccepted] = useState(false)
  const [result, setResult] = useState<KeyAccountResult | null>(null)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const [copyError, setCopyError] = useState('')
  const mutation = useManageAccountKey({
    mutation: {
      gcTime: 0,
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        try {
          sessionStorage.removeItem(storageKey)
          setPending(null)
        } catch {
          setStorageError(
            'Confirmed, but the request could not be cleared from this browser. No key was stored.',
          )
        }
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        void client.invalidateQueries({ queryKey: getGetAccountQueryKey(value.user_id) })
      },
      onError: (error) => {
        setFailure(errorText(error))
        setRefused(
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status),
        )
        if (error instanceof ApiHttpError && [401, 403].includes(error.status))
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (result || mutation.isPending || storageError) return
    let variables = pending
    if (!variables) {
      if (!action || !accepted) return
      const data =
        action.kind === 'create'
          ? {
              kind: 'create' as const,
              names: names.trim() ? names.split(',').map((n) => n.trim()) : [],
            }
          : action
      variables = { data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      if (!pendingSchema.safeParse(variables).success) {
        setFailure('Use at most four player names.')
        return
      }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this request safely. No request was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  function close() {
    if (mutation.isPending) return
    setResult(null)
    mutation.reset()
    setNames('')
    setAccepted(false)
    if (!pending || result) {
      setFailure('')
      setRefused(false)
    }
    setCopyError('')
    onChange(null)
  }
  const create = (pending?.data ?? action)?.kind === 'create'
  const title = create ? 'Create key account' : 'Reveal account key'
  return (
    <>
      <Button
        disabled={session.role !== 'admin' || !!storageError}
        onClick={() => onChange(pending ? pending.data : { kind: 'create' })}
      >
        <KeyRound size={15} /> {pending ? 'Resume key request' : 'Create key account'}
      </Button>
      {storageError && (
        <p role="alert" className="connected-error">
          {storageError}
        </p>
      )}
      <Modal
        open={action !== null}
        onOpenChange={(value) => {
          if (!value) close()
        }}
        title={title}
        description="Private game credential. Creation and disclosure are audited."
      >
        <form id="connected-key" className="modal-body" onSubmit={submit}>
          {failure && (
            <p role="alert" className="connected-error">
              {failure}
            </p>
          )}
          {result ? (
            <>
              <p role="status">
                Account #{result.user_id}
                {result.players.length > 0 && ` · ${result.players.map((p) => p.name).join(', ')}`}
              </p>
              <Field id="account-key" label="Account key">
                <input
                  id="account-key"
                  readOnly
                  value={result.key}
                  autoComplete="off"
                  spellCheck={false}
                />
              </Field>
              <p className="field-hint">
                Share privately with the account owner. This key allows game access without a
                password. It is cleared from this dialog when closed.
              </p>
              <Button
                onClick={() => {
                  void (async () => {
                    try {
                      await navigator.clipboard.writeText(result.key)
                      setCopyError('')
                    } catch {
                      setCopyError('Could not copy. Select the key above and copy it manually.')
                    }
                  })()
                }}
              >
                <Clipboard size={15} /> Copy key
              </Button>
              {copyError && (
                <p role="alert" className="connected-error">
                  {copyError}
                </p>
              )}
            </>
          ) : pending ? (
            <p className="inline-notice">
              {create
                ? 'Account creation'
                : `Key request for account #${pending.data.kind === 'reveal' ? pending.data.user_id : ''}`}{' '}
              is unconfirmed. Retry the original request to retrieve its result safely.
            </p>
          ) : (
            <>
              {create ? (
                <Field id="key-player-names" label="Player names">
                  <input
                    id="key-player-names"
                    value={names}
                    onChange={(e) => setNames(e.target.value)}
                    placeholder="Kaiba, Yugi"
                    autoComplete="off"
                  />
                  <p className="field-hint">
                    Up to four names, separated by commas, 1–16 characters each. Leave empty for the
                    host's starting players.
                  </p>
                </Field>
              ) : (
                <p>
                  Reveal or assign the game key for account #
                  {action?.kind === 'reveal' ? action.user_id : ''}.
                </p>
              )}
              <label className="profile-role-confirm">
                <input
                  type="checkbox"
                  checked={accepted}
                  onChange={(e) => setAccepted(e.target.checked)}
                />{' '}
                I understand this key allows passwordless game access
              </label>
            </>
          )}
          {pending && (
            <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions">
          <Button onClick={close} disabled={mutation.isPending}>
            {result ? 'Done' : 'Close'}
          </Button>
          {!result && refused && (
            <Button
              onClick={() => {
                try {
                  sessionStorage.removeItem(storageKey)
                  if (pending?.data.kind === 'create') setNames(pending.data.names.join(', '))
                  setPending(null)
                  setFailure('')
                  setRefused(false)
                } catch {
                  setStorageError('Could not clear this request. No new request will be sent.')
                }
              }}
            >
              Edit request
            </Button>
          )}
          {!result && (
            <Button
              variant="primary"
              type="submit"
              form="connected-key"
              disabled={mutation.isPending || !!storageError || (!pending && !accepted) || refused}
            >
              {mutation.isPending && <LoaderCircle size={15} className="animate-spin" />}
              {mutation.isPending
                ? 'Confirming…'
                : pending
                  ? 'Retry same request'
                  : create
                    ? 'Create and reveal key'
                    : 'Reveal account key'}
            </Button>
          )}
        </div>
      </Modal>
    </>
  )
}
