import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, Trash2 } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal } from './components/ui'
import {
  getListAccountsQueryKey,
  getGetStaffSessionQueryKey,
  useDeleteAccount,
  useGetAccount,
} from './api/generated/http/client'
import type { DeleteAccountMutationVariables } from './api/generated/http/client'
import type { DeleteAccountResponse, StaffSession } from './api/generated/http/models'
import { DeleteAccountHeader, DeleteAccountRequest } from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'
const pendingSchema = z.object({ data: DeleteAccountRequest, headers: DeleteAccountHeader })
export function ConnectedDeletion({
  session,
  userId,
  onChange,
}: {
  session: StaffSession
  userId: number | null
  onChange: (id: number | null) => void
}) {
  const client = useQueryClient()
  const key = `yo3:pending-deletion:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(key)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error:
          'Could not restore a deletion request. Check the audit log before deleting another account.',
      }
    }
  })
  const [pending, setPending] = useState<DeleteAccountMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [confirmation, setConfirmation] = useState('')
  const [result, setResult] = useState<DeleteAccountResponse | null>(null)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const target = pending?.data.user_id ?? userId
  const account = useGetAccount(target ?? 0, {
    query: { enabled: target !== null && !pending && !result },
  })
  function clearIntent() {
    try {
      sessionStorage.removeItem(key)
      setPending(null)
      return true
    } catch {
      setStorageError('Could not clear this request. No new deletion was sent.')
      return false
    }
  }
  const mutation = useDeleteAccount({
    mutation: {
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        if (value.disconnect !== 'pending' && value.disconnect !== 'interrupted') clearIntent()
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        if (value.result.user_id === session.user_id)
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
      onError: (error) => {
        setFailure(
          error instanceof ApiHttpError
            ? error.message
            : 'Could not confirm deletion. Retry this same request to find out what happened.',
        )
        setRefused(
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status),
        )
        if (error instanceof ApiHttpError && [401, 403].includes(error.status))
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
    },
  })
  const busy = mutation.isPending
  function close() {
    if (busy || (result && !clearIntent())) return
    setResult(null)
    setFailure('')
    setRefused(false)
    setConfirmation('')
    mutation.reset()
    onChange(null)
  }
  function edit() {
    if (!refused || !clearIntent()) return
    setRefused(false)
    setFailure('')
    setConfirmation('')
    mutation.reset()
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || result || storageError || refused) return
    let variables = pending
    if (!variables) {
      if (
        !target ||
        session.role !== 'admin' ||
        account.isFetching ||
        account.error ||
        !account.data ||
        confirmation !== String(target)
      )
        return
      variables = {
        data: { user_id: target, confirm_user_id: Number(confirmation) },
        headers: { 'Idempotency-Key': crypto.randomUUID() },
      }
      try {
        sessionStorage.setItem(key, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this request safely. No deletion was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  return (
    <>
      {pending && userId === null && (
        <Button onClick={() => onChange(pending.data.user_id)}>
          <Trash2 size={15} /> Resume account deletion
        </Button>
      )}
      {storageError && userId === null && (
        <p role="alert" className="connected-error">
          {storageError}
        </p>
      )}
      <Modal
        open={userId !== null}
        onOpenChange={(open) => {
          if (!open) close()
        }}
        title={`Delete account #${target ?? ''}`}
        description="Permanently removes this account and its saved game data."
      >
        <form id="connected-deletion" onSubmit={submit} className="modal-body">
          {result ? (
            <>
              <p role="status" className="inline-notice">
                {result.result.message}
              </p>
              <p>{result.result.players} player(s) removed.</p>
              {result.disconnect === 'completed' && <p>This account’s sessions were ended.</p>}
              {(result.disconnect === 'pending' || result.disconnect === 'interrupted') && (
                <p role="alert" className="connected-error">
                  The account is deleted. Its disconnect outcome could not be confirmed. Check
                  current sessions; retrying this request only checks the saved outcome.
                </p>
              )}
            </>
          ) : pending ? (
            <p className="inline-notice">
              Deletion of account #{pending.data.user_id} needs confirmation from the server. Retry
              this same request, even if the account has disappeared from the list.
            </p>
          ) : (
            <>
              {account.isPending && (
                <p role="status" className="connected-state">
                  <LoaderCircle size={16} className="spin" /> Loading account…
                </p>
              )}
              {account.error && (
                <div role="alert" className="connected-error">
                  <p>{account.error.message}</p>
                  <Button onClick={() => void account.refetch()} disabled={account.isFetching}>
                    Try again
                  </Button>
                </div>
              )}
              {account.data && (
                <>
                  <p>
                    <strong>
                      {account.data.login ?? 'Key account'} · #{target}
                    </strong>
                  </p>
                  <p>
                    Removes all players, cards, decks, devices, mail, social and team membership,
                    hosted events, saved records and duel logs for this account. Its game will be
                    disconnected. This cannot be undone here.
                  </p>
                  <p>Backups and logs already exported may retain older data.</p>
                  {target === session.user_id && (
                    <p role="alert" className="connected-error">
                      This is your own account. Deleting it signs you out. The last administrator
                      cannot be deleted.
                    </p>
                  )}
                  <Field label={`Type ${target} to confirm deletion`} id="delete-account-confirm">
                    <input
                      id="delete-account-confirm"
                      inputMode="numeric"
                      autoComplete="off"
                      value={confirmation}
                      onChange={(e) => setConfirmation(e.target.value)}
                      disabled={busy || session.role !== 'admin'}
                    />
                  </Field>
                </>
              )}
            </>
          )}
          {session.role !== 'admin' && (
            <p role="alert" className="connected-error">
              Only an administrator can delete accounts.
            </p>
          )}
          {storageError && (
            <p role="alert" className="connected-error">
              {storageError}
            </p>
          )}
          {failure && (
            <p role="alert" className="connected-error">
              {failure}
            </p>
          )}
        </form>
        <div className="modal-actions">
          <Button onClick={close} disabled={busy}>
            {result ? 'Done' : 'Cancel'}
          </Button>
          {result && pending && (
            <Button onClick={() => mutation.mutate(pending)} disabled={busy}>
              Check outcome
            </Button>
          )}
          {pending && refused && <Button onClick={edit}>Edit request</Button>}
          {!result && (
            <Button
              form="connected-deletion"
              type="submit"
              variant="danger"
              disabled={
                busy ||
                Boolean(storageError) ||
                refused ||
                session.role !== 'admin' ||
                (!pending &&
                  (confirmation !== String(target) ||
                    !account.data ||
                    account.isFetching ||
                    Boolean(account.error)))
              }
            >
              {busy ? (
                <>
                  <LoaderCircle size={16} className="spin" /> Deleting…
                </>
              ) : pending ? (
                'Retry same deletion'
              ) : (
                'Delete account permanently'
              )}
            </Button>
          )}
        </div>
      </Modal>
    </>
  )
}
