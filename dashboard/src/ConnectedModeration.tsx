import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, ShieldCheck } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal } from './components/ui'
import {
  getGetAccountQueryKey,
  getGetStaffSessionQueryKey,
  getListAccountsQueryKey,
  useGetAccount,
  useModerateAccount,
} from './api/generated/http/client'
import type { ModerateAccountMutationVariables } from './api/generated/http/client'
import type {
  ModerationRequest,
  ModerationResponse,
  StaffSession,
} from './api/generated/http/models'
import {
  ModerateAccountHeader,
  ModerationRequest as RequestSchema,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

const pendingSchema = z.object({ data: RequestSchema, headers: ModerateAccountHeader })
const actions: Record<ModerationRequest['kind'], { label: string; help: string }> = {
  mute: {
    label: 'Mute chat',
    help: 'Blocks game chat and D-Mail until the mute ends or is lifted. Their game stays connected.',
  },
  unmute: { label: 'Lift mute', help: 'Allows this account to use chat and D-Mail again.' },
  kick: {
    label: 'Disconnect game',
    help: 'Disconnects their current game. They can sign in again. A retry never disconnects a newly reconnected game.',
  },
  restrict: {
    label: 'Restrict sign-in',
    help: 'Refuses their next sign-in. A game already connected can keep playing.',
  },
  lift: {
    label: 'Lift ban or restriction',
    help: 'Allows this account to sign in again. Signed-out devices must sign in again.',
  },
  ban: {
    label: 'Ban account',
    help: 'Refuses sign-in, signs out their saved devices and disconnects their game.',
  },
  sign_out: {
    label: 'Sign out everywhere',
    help: 'Signs out saved devices, staff sessions and their current game. They can sign in again.',
  },
}
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Retry this same action to confirm what happened.'
export function ConnectedModeration({
  session,
  userId,
  onChange,
}: {
  session: StaffSession
  userId: number | null
  onChange: (id: number | null) => void
}) {
  const client = useQueryClient()
  const key = `yo3:pending-moderation:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(key)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error:
          'Could not restore a moderation action. Check the audit log before starting another.',
      }
    }
  })
  const [pending, setPending] = useState<ModerateAccountMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [kind, setKind] = useState<ModerationRequest['kind']>('mute')
  const [reason, setReason] = useState('')
  const [duration, setDuration] = useState('')
  const [accepted, setAccepted] = useState(false)
  const [result, setResult] = useState<ModerationResponse | null>(null)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const target = pending?.data.user_id ?? userId
  const account = useGetAccount(target ?? 0, { query: { enabled: target !== null } })
  function clearIntent() {
    try {
      sessionStorage.removeItem(key)
      setPending(null)
      return true
    } catch {
      setStorageError('Could not clear this request. No new moderation action was sent.')
      return false
    }
  }
  const mutation = useModerateAccount({
    mutation: {
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        // A received confirmation proves the stored change. Keep an uncertain effect's receipt until Done.
        if (value.disconnect !== 'pending' && value.disconnect !== 'interrupted') clearIntent()
        void client.invalidateQueries({ queryKey: getGetAccountQueryKey(value.result.user_id) })
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        if (
          value.result.user_id === session.user_id &&
          ['ban', 'restrict', 'sign-out'].includes(value.result.operation)
        )
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
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
  const currentKind = pending?.data.kind ?? (account.data && !account.data.login ? 'kick' : kind)
  const mayAct =
    session.role === 'admin' ||
    Boolean(
      account.data && (account.data.role === 'player' || account.data.user_id === session.user_id),
    )
  const needsReason = ['ban', 'restrict', 'mute'].includes(currentKind)
  const needsDuration = needsReason
  const busy = mutation.isPending
  function close() {
    if (busy) return
    if (result && !clearIntent()) return
    setResult(null)
    setFailure('')
    setRefused(false)
    setAccepted(false)
    setKind('mute')
    setReason('')
    setDuration('')
    mutation.reset()
    onChange(null)
  }
  function edit() {
    if (!pending || !refused) return
    const request = pending.data
    setKind(request.kind)
    setReason('reason' in request ? request.reason : '')
    setDuration(
      'minutes' in request
        ? String(request.minutes ?? '')
        : 'days' in request
          ? String(request.days ?? '')
          : '',
    )
    if (clearIntent()) {
      setFailure('')
      setRefused(false)
      setAccepted(false)
      mutation.reset()
    }
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || storageError || result) return
    let variables = pending
    if (!variables) {
      if (!target || !mayAct || !accepted || !account.data || account.error || account.isFetching)
        return
      const base = { kind: currentKind, user_id: target }
      const data =
        currentKind === 'ban' || currentKind === 'restrict'
          ? { ...base, kind: currentKind, reason, days: duration ? Number(duration) : null }
          : currentKind === 'mute'
            ? { ...base, kind: currentKind, reason, minutes: duration ? Number(duration) : null }
            : base
      const parsed = RequestSchema.safeParse(data)
      if (!parsed.success) {
        setFailure(
          'Use a whole, positive duration or leave it empty. The reason is at most 200 characters.',
        )
        return
      }
      variables = { data: parsed.data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        sessionStorage.setItem(key, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this action safely. No request was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  const shownReason = pending && 'reason' in pending.data ? pending.data.reason : reason
  const shownDuration =
    pending && 'minutes' in pending.data
      ? String(pending.data.minutes ?? '')
      : pending && 'days' in pending.data
        ? String(pending.data.days ?? '')
        : duration
  return (
    <>
      {pending && userId === null && (
        <Button onClick={() => onChange(pending.data.user_id)}>
          <ShieldCheck size={15} /> Resume moderation action
        </Button>
      )}
      {storageError && (
        <p role="alert" className="connected-error">
          {storageError}
        </p>
      )}
      <Modal
        open={userId !== null}
        onOpenChange={(value) => {
          if (!value) close()
        }}
        title={`Moderate ${account.data?.login ?? `account #${target ?? ''}`}`}
        description="Changes and disconnect outcomes are recorded in the audit log. Review the effect before confirming."
      >
        <form id="connected-moderation" onSubmit={submit} className="modal-body">
          {result ? (
            <>
              <p role="status" className="inline-notice notice-success">
                {result.result.message}
              </p>
              {result.result.until && (
                <p>Until {new Date(result.result.until).toLocaleString()}.</p>
              )}
              {result.disconnect === 'completed' && (
                <p>
                  The server ended this account’s game sessions. A newly connected game is left
                  alone on retry.
                </p>
              )}
              {(result.disconnect === 'pending' || result.disconnect === 'interrupted') && (
                <div role="alert" className="inline-notice notice-warning">
                  The account change is saved, but the disconnect outcome is unknown
                  {result.disconnect === 'interrupted' ? ' after an interruption' : ''}. Check their
                  current session before requesting another disconnect. Retrying this receipt will
                  only check its outcome.
                </div>
              )}
              {pending && (
                <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
              )}
            </>
          ) : (
            <>
              {account.isPending && (
                <p className="connected-state">
                  <LoaderCircle size={16} className="spin" /> Loading account…
                </p>
              )}
              {account.error && (
                <div role="alert" className="connected-error">
                  <p>{errorText(account.error)}</p>
                  <Button onClick={() => void account.refetch()} disabled={account.isFetching}>
                    Refresh account
                  </Button>
                </div>
              )}
              {account.data && !mayAct && (
                <p className="connected-error">Moderators cannot act on another staff account.</p>
              )}
              {(account.data || pending) && (
                <fieldset
                  className="settings-fieldset"
                  disabled={busy || Boolean(pending) || !mayAct}
                >
                  <Field id="moderation-action" label="Action">
                    <select
                      id="moderation-action"
                      value={currentKind}
                      onChange={(e) => {
                        setKind(e.target.value as ModerationRequest['kind'])
                        setAccepted(false)
                      }}
                    >
                      {Object.entries(actions).map(([value, action]) => (
                        <option
                          key={value}
                          value={value}
                          disabled={!account.data?.login && value !== 'kick'}
                        >
                          {action.label}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <p className="inline-notice">{actions[currentKind].help}</p>
                  {target === session.user_id && (
                    <p className="connected-error">
                      This is your account. Banning, restricting or signing out everywhere ends your
                      management session too.
                    </p>
                  )}
                  {needsReason && (
                    <Field
                      id="moderation-reason"
                      label="Reason"
                      hint="Shown to the player. Optional; up to 200 characters on one line."
                    >
                      <input
                        id="moderation-reason"
                        value={shownReason}
                        maxLength={200}
                        onChange={(e) => {
                          setReason(e.target.value)
                          setAccepted(false)
                        }}
                      />
                    </Field>
                  )}
                  {needsDuration && (
                    <Field
                      id="moderation-duration"
                      label={currentKind === 'mute' ? 'Duration (minutes)' : 'Duration (days)'}
                      hint="Leave empty until a staff member lifts it."
                    >
                      <input
                        id="moderation-duration"
                        type="number"
                        min={1}
                        max={4294967295}
                        step={1}
                        value={shownDuration}
                        onChange={(e) => {
                          setDuration(e.target.value)
                          setAccepted(false)
                        }}
                      />
                    </Field>
                  )}
                  <label className="profile-role-confirm">
                    <input
                      type="checkbox"
                      checked={accepted}
                      onChange={(e) => setAccepted(e.target.checked)}
                    />{' '}
                    Confirm this action for this account
                  </label>
                </fieldset>
              )}
              {pending && (
                <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
              )}
            </>
          )}
          {failure && (
            <div role="alert" className="connected-error">
              <p>{failure}</p>
              {pending && !refused && (
                <p>Retry this same action; a disconnect will never be repeated.</p>
              )}
            </div>
          )}
        </form>
        <div className="modal-actions">
          <Button onClick={close} disabled={busy}>
            {result ? 'Done' : 'Close'}
          </Button>
          {result && pending && (
            <Button onClick={() => mutation.mutate(pending)} disabled={busy}>
              Check outcome
            </Button>
          )}
          {pending && refused && <Button onClick={edit}>Edit request</Button>}
          {!result && (
            <Button
              type="submit"
              form="connected-moderation"
              variant={['ban', 'kick', 'sign_out'].includes(currentKind) ? 'danger' : 'primary'}
              disabled={
                busy ||
                Boolean(storageError) ||
                Boolean(pending && refused) ||
                (!pending &&
                  (!mayAct ||
                    !accepted ||
                    account.isFetching ||
                    Boolean(account.error) ||
                    !account.data))
              }
            >
              {busy ? (
                <>
                  <LoaderCircle size={16} className="spin" /> Applying…
                </>
              ) : pending ? (
                'Retry same action'
              ) : (
                'Apply action'
              )}
            </Button>
          )}
        </div>
      </Modal>
    </>
  )
}
