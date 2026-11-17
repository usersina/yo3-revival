import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Clipboard, LoaderCircle, Plus, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  getListInvitesQueryKey,
  useCreateInvite,
  useListInvites,
} from './api/generated/http/client'
import type { CreateInviteMutationVariables } from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import { CreateInviteHeader, CreateInviteRequest } from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

const pendingSchema = z.object({ data: CreateInviteRequest, headers: CreateInviteHeader })
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry the same request.'

export function ConnectedInvites({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const invites = useListInvites()
  const storageKey = `yo3:pending-invite:${session.user_id}`
  const [stored, setStored] = useState<{
    intent: CreateInviteMutationVariables | null
    error: string | null
  }>(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { intent: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: null }
    } catch {
      return {
        intent: null,
        error:
          'Could not restore a saved invite request. Review the server audit log before creating another.',
      }
    }
  })
  const [open, setOpen] = useState(false)
  const [uses, setUses] = useState('1')
  const [days, setDays] = useState('7')
  const [notice, setNotice] = useState('')
  const [failure, setFailure] = useState('')
  const [uncertain, setUncertain] = useState(false)
  const create = useCreateInvite({
    mutation: {
      onSuccess: (result) => {
        try {
          sessionStorage.removeItem(storageKey)
          setStored({ intent: null, error: null })
        } catch {
          setStored((previous) => ({
            ...previous,
            error:
              'Saved, but this browser could not clear the request. Reload to confirm the same invite safely.',
          }))
        }
        setNotice(`Invite created: ${result.code}. Give it to the player for account registration.`)
        setFailure('')
        setOpen(false)
        void client.invalidateQueries({ queryKey: getListInvitesQueryKey() })
      },
      onError: (error) => {
        const refused =
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status)
        setUncertain(!refused)
        setFailure(errorText(error))
        if (refused) {
          try {
            sessionStorage.removeItem(storageKey)
            setStored({ intent: null, error: null })
          } catch {
            setStored((previous) => ({
              ...previous,
              error: 'Could not clear the refused request. Reload before creating another invite.',
            }))
          }
        }
        // Auth and CSRF refusals precede receipt lookup; an earlier attempt may have committed.
        if (error instanceof ApiHttpError && [401, 403].includes(error.status)) {
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
        }
      },
    },
  })
  const allowed = session.role === 'admin' || session.role === 'moderator'
  const busy = create.isPending
  const draft = CreateInviteRequest.safeParse({
    uses: uses.trim() ? Number(uses) : null,
    days: days.trim() ? Number(days) : null,
  })

  function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || !allowed || stored.error) return
    let intent = stored.intent
    if (!intent) {
      if (!draft.success) return
      intent = { data: draft.data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(intent))
      } catch {
        setStored({
          intent: null,
          error:
            'This browser could not retain the request ID. Enable session storage before creating invites.',
        })
        return
      }
      setStored({ intent, error: null })
    }
    setFailure('')
    create.mutate(intent)
  }

  return (
    <section aria-label="Registration invites" className="connected-invites">
      <Panel
        title="Registration invites"
        description="Players enter these when creating an account if registration requires an invite. Admins and moderators can issue them."
        action={
          <Button
            variant="primary"
            disabled={!allowed || busy || Boolean(stored.intent) || Boolean(stored.error)}
            onClick={() => {
              setUses('1')
              setDays('7')
              setNotice('')
              setFailure('')
              setOpen(true)
            }}
          >
            <Plus size={16} />
            Create invite
          </Button>
        }
      >
        {notice && (
          <p className="inline-notice" role="status">
            {notice}
          </p>
        )}
        {stored.error && (
          <p className="connected-error" role="alert">
            {stored.error}
          </p>
        )}
        {stored.intent && (
          <div className="inline-notice">
            <div>
              <strong>Unconfirmed invite</strong>
              <p>It may already exist. Resume the same request before creating another.</p>
            </div>
            <Button disabled={busy} onClick={() => setOpen(true)}>
              Resume invite
            </Button>
          </div>
        )}
        <div className="table-toolbar">
          <span className="muted">
            {invites.data
              ? `Status checked ${new Date(invites.data.checked_at).toLocaleString()}`
              : 'Invite list'}
          </span>
          <Button onClick={() => void invites.refetch()} disabled={invites.isFetching}>
            <RefreshCw size={15} />
            Refresh invites
          </Button>
        </div>
        {invites.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading invites…
          </p>
        )}
        {invites.error && (
          <p role="alert" className="connected-error">
            The invite list could not be refreshed. {errorText(invites.error)}
            {invites.error instanceof ApiHttpError && invites.error.status === 401 && (
              <Button
                onClick={() =>
                  void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
                }
              >
                Sign in again
              </Button>
            )}
          </p>
        )}
        {invites.data && (
          <>
            <div
              className="table-scroll"
              role="region"
              aria-label="Registration invites table"
              tabIndex={0}
            >
              <table>
                <thead>
                  <tr>
                    <th scope="col">Code</th>
                    <th scope="col">Status</th>
                    <th scope="col">Sign-ups left</th>
                    <th scope="col">Expires</th>
                    <th scope="col">Issued by</th>
                    <th scope="col">Used by</th>
                  </tr>
                </thead>
                <tbody>
                  {[...invites.data.invites].reverse().map((invite) => (
                    <tr key={invite.code}>
                      <td>
                        <div className="code-cell">
                          <code>{invite.code}</code>
                          <Button
                            variant="ghost"
                            className="icon-button"
                            aria-label={`Copy invite ${invite.code}`}
                            onClick={async () => {
                              try {
                                await navigator.clipboard.writeText(invite.code)
                                setNotice('Invite copied.')
                              } catch {
                                setNotice(
                                  'Could not copy. Select the invite code and copy it manually.',
                                )
                              }
                            }}
                          >
                            <Clipboard size={15} />
                          </Button>
                        </div>
                      </td>
                      <td>
                        <Badge tone={invite.state === 'usable' ? 'green' : 'neutral'}>
                          {invite.state === 'usable'
                            ? 'Usable'
                            : invite.state === 'expired'
                              ? 'Expired'
                              : 'Used up'}
                        </Badge>
                      </td>
                      <td className="numeric">
                        {invite.uses == null ? 'Unlimited' : invite.uses.toLocaleString()}
                      </td>
                      <td>
                        {invite.expires_at ? (
                          <time dateTime={invite.expires_at}>
                            {new Date(invite.expires_at).toLocaleString()}
                          </time>
                        ) : (
                          'No expiry'
                        )}
                      </td>
                      <td>{invite.issued_by}</td>
                      <td>
                        {invite.claims.length ? (
                          <details>
                            <summary>{invite.claims.length} accounts</summary>
                            <ul>
                              {invite.claims.map((claim) => (
                                <li key={claim.user_id}>
                                  {claim.players.join(', ') || `Account #${claim.user_id}`}
                                </li>
                              ))}
                            </ul>
                          </details>
                        ) : (
                          'None'
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {invites.data.invites.length === 0 && (
              <p className="connected-state">No registration invites yet.</p>
            )}
          </>
        )}
      </Panel>
      <Modal
        open={open}
        onOpenChange={(value) => {
          if (!busy) setOpen(value)
        }}
        title="Create registration invite"
        description="Choose how many accounts can register and when the invite expires. A retry keeps the original code and expiry."
      >
        <form id="connected-invite" onSubmit={submit} className="modal-body">
          <Field id="invite-uses" label="Sign-up limit" hint="Leave empty for unlimited sign-ups.">
            <input
              id="invite-uses"
              type="number"
              min={1}
              max={4294967295}
              step={1}
              value={stored.intent ? (stored.intent.data.uses ?? '') : uses}
              onChange={(e) => setUses(e.target.value)}
              disabled={busy || Boolean(stored.intent)}
            />
          </Field>
          <Field
            id="invite-days"
            label="Expires after (days)"
            hint="Leave empty for no expiry. Starts when the invite is created."
          >
            <input
              id="invite-days"
              type="number"
              min={1}
              max={4294967295}
              step={1}
              value={stored.intent ? (stored.intent.data.days ?? '') : days}
              onChange={(e) => setDays(e.target.value)}
              disabled={busy || Boolean(stored.intent)}
            />
          </Field>
          {failure && (
            <div role="alert" className="connected-error">
              <p>{uncertain ? 'We could not confirm this invite.' : 'The invite was refused.'}</p>
              <p>{failure}</p>
              {stored.intent && <p>Retry this same request to avoid creating a second invite.</p>}
            </div>
          )}
          {stored.intent && (
            <p className="field-hint">Request ID: {stored.intent.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">
            {stored.intent ? 'Request saved for safe retry' : 'Admins and moderators'}
          </span>
          <div className="action-group">
            <Button disabled={busy} onClick={() => setOpen(false)}>
              Close
            </Button>
            <Button
              variant="primary"
              type="submit"
              form="connected-invite"
              disabled={
                !allowed || busy || Boolean(stored.error) || (!stored.intent && !draft.success)
              }
            >
              {busy && <LoaderCircle size={16} className="animate-spin" />}
              {busy ? 'Creating…' : stored.intent ? 'Retry same invite' : 'Create invite'}
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  )
}
