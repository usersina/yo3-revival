import { useState } from 'react'
import type { FormEvent } from 'react'
import { useSearchParams } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { Coins, Eye, LoaderCircle, LogOut, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Badge, Button, Field, Modal, PageHeading, Panel, SearchField } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  getListAccountsQueryKey,
  useGiveBp,
  useListAccounts,
  useSignOutStaff,
} from './api/generated/http/client'
import type { GiveBpMutationVariables } from './api/generated/http/client'
import type { AccountSummary, StaffSession } from './api/generated/http/models'
import { GiveBpHeader, GiveBpByIdRequest } from './api/generated/validation/schemas'
import { ApiHttpError, setCsrfToken } from './api/transport'
import { ConnectedAccountDialog } from './ConnectedAccountDialog'
import { ConnectedModeration } from './ConnectedModeration'
import { ConnectedDeletion } from './ConnectedDeletion'
import { ConnectedSettings } from './ConnectedSettings'
import { ConnectedServer } from './ConnectedServer'
import { ConnectedAudit } from './ConnectedAudit'
import { ConnectedLogs } from './ConnectedLogs'
import { ConnectedBackups } from './ConnectedBackups'
import { ConnectedCodes } from './ConnectedCodes'
import { ConnectedInvites } from './ConnectedInvites'
import { ConnectedGifts } from './ConnectedGifts'
import type { GiftTarget } from './ConnectedGifts'
import { ConnectedKeys } from './ConnectedKeys'
import type { KeyAction } from './ConnectedKeys'
import { ConnectedCredentials } from './ConnectedCredentials'
import type { CredentialAction } from './ConnectedCredentials'

const pendingSchema = z.object({ data: GiveBpByIdRequest, headers: GiveBpHeader })

function errorText(error: unknown) {
  return error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check that it is running, then try again.'
}

export default function ConnectedAccounts({
  session,
  full = true,
  title = 'Accounts',
}: {
  session: StaffSession
  full?: boolean
  title?: string
}) {
  const client = useQueryClient()
  const accounts = useListAccounts()
  const [query, setQuery] = useState('')
  const [standingFilter, setStandingFilter] = useState('all')
  const [deletionId, setDeletionId] = useState<number | null>(null)
  const [moderationId, setModerationId] = useState<number | null>(null)
  const [params, setParams] = useSearchParams()
  const rawAccount = params.get('account')
  const [integrationDetailId, setIntegrationDetailId] = useState<number | null>(null)
  const linkedDetailId =
    rawAccount && /^[1-9][0-9]{0,9}$/.test(rawAccount) && Number(rawAccount) <= 4294967295
      ? Number(rawAccount)
      : null
  const detailId = full ? integrationDetailId : linkedDetailId
  function setDetailId(value: number | null) {
    if (full) {
      setIntegrationDetailId(value)
      return
    }
    const next = new URLSearchParams(params)
    if (value === null) next.delete('account')
    else next.set('account', String(value))
    setParams(next, { preventScrollReset: true })
  }
  const [giftTarget, setGiftTarget] = useState<GiftTarget | null>(null)
  const [keyAction, setKeyAction] = useState<KeyAction | null>(null)
  const [credentialAction, setCredentialAction] = useState<CredentialAction | null>(null)
  const [selected, setSelected] = useState<AccountSummary | null>(null)
  const [amount, setAmount] = useState('500')
  const [notice, setNotice] = useState('')
  const storageKey = `yo3:pending-grant:${session.user_id}`
  const [stored, setStored] = useState<{
    intent: GiveBpMutationVariables | null
    error: string | null
  }>(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { intent: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: null }
    } catch {
      return {
        intent: null,
        error:
          'Could not restore a previous grant. Review the server audit log before starting another.',
      }
    }
  })
  const [open, setOpen] = useState(false)
  const grant = useGiveBp({
    mutation: {
      onError: (error) => {
        // Auth/CSRF refusals precede receipt lookup: an earlier attempt may already have committed.
        if (error instanceof ApiHttpError && [401, 403].includes(error.status)) {
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
        }
        if (error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status)) {
          try {
            sessionStorage.removeItem(storageKey)
            setStored({ intent: null, error: null })
          } catch {
            setStored((previous) => ({
              ...previous,
              error: 'Could not clear this refused request. Reload before starting another grant.',
            }))
          }
        }
      },
      onSuccess: () => {
        // Keep the receipt key until the server confirms the response, including after a reload.
        try {
          sessionStorage.removeItem(storageKey)
          setStored({ intent: null, error: null })
        } catch {
          setStored((previous) => ({
            ...previous,
            error:
              'BP added, but this browser could not clear the saved request. Reload to confirm it again safely.',
          }))
        }
        setNotice('BP added. The balance, D-Mail and audit log were saved together.')
        setOpen(false)
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
      },
    },
  })
  const signOut = useSignOutStaff({
    mutation: {
      onSuccess: () => {
        setCsrfToken(undefined)
        void client.resetQueries()
      },
    },
  })
  const validAmount = GiveBpByIdRequest.safeParse({
    user_id: selected?.user_id ?? 0,
    amount: Number(amount),
  }).success
  function submit(event: FormEvent) {
    event.preventDefault()
    if (grant.isPending || session.role !== 'admin' || stored.error) return
    let intent = stored.intent
    if (!intent) {
      if (!selected || !validAmount) return
      intent = {
        data: { user_id: selected.user_id, amount: Number(amount) },
        headers: { 'Idempotency-Key': crypto.randomUUID() },
      }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(intent))
      } catch {
        setStored({
          intent: null,
          error:
            'This browser could not retain the request ID. Enable session storage before giving BP.',
        })
        return
      }
      setStored({ intent, error: null })
    }
    grant.mutate(intent)
  }
  if (accounts.error instanceof ApiHttpError && accounts.error.status === 401)
    return (
      <Panel title="Session ended" description="Returning to sign-in…">
        <p role="status">Your session has ended.</p>
      </Panel>
    )
  const filtered = accounts.data?.accounts.filter(
    (a) =>
      `${a.login ?? ''} ${a.user_id} ${a.players.map((p) => p.name).join(' ')}`
        .toLowerCase()
        .includes(query.trim().toLowerCase()) &&
      (standingFilter === 'all' ||
        (standingFilter === 'muted' ? a.muted != null : a.standing?.kind === standingFilter)),
  )
  return (
    <>
      <PageHeading
        title={title}
        description={`Signed in as ${session.user} · ${session.role}`}
        action={
          full && (
            <Button
              onClick={() =>
                signOut.mutate({ headers: { 'Idempotency-Key': crypto.randomUUID() } })
              }
              disabled={signOut.isPending || grant.isPending}
            >
              <LogOut size={16} />
              {signOut.isPending ? 'Signing out…' : 'Sign out'}
            </Button>
          )
        }
      />
      {signOut.error && (
        <p className="connected-error" role="alert">
          {errorText(signOut.error)}
        </p>
      )}
      {!full && rawAccount !== null && detailId === null && (
        <p role="alert" className="connected-error">
          This account link is invalid. Choose an account from the list.
        </p>
      )}
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
            <strong>Unconfirmed grant</strong>
            <p>
              {stored.intent.data.amount.toLocaleString()} BP to account #
              {stored.intent.data.user_id}. Confirm this request before creating another.
            </p>
          </div>
          <Button onClick={() => setOpen(true)}>Resume grant</Button>
        </div>
      )}
      {full && <ConnectedServer session={session} />}
      <Panel>
        <div className="table-toolbar">
          <SearchField
            label="Search accounts"
            placeholder="User ID or player name…"
            value={query}
            onChange={setQuery}
          />
          <Field id="accounts-standing-filter" label="Standing">
            <select
              id="accounts-standing-filter"
              value={standingFilter}
              onChange={(event) => setStandingFilter(event.target.value)}
            >
              <option value="all">All accounts</option>
              <option value="banned">Banned</option>
              <option value="restricted">Restricted</option>
              <option value="muted">Muted</option>
            </select>
          </Field>
          <div className="action-group">
            <ConnectedDeletion session={session} userId={deletionId} onChange={setDeletionId} />
            <ConnectedModeration
              session={session}
              userId={moderationId}
              onChange={setModerationId}
            />
            <ConnectedGifts session={session} target={giftTarget} onChange={setGiftTarget} />
            <ConnectedKeys session={session} action={keyAction} onChange={setKeyAction} />
            <ConnectedCredentials
              key={`${session.user_id}:${credentialAction?.kind ?? 'closed'}:${credentialAction?.kind === 'reset_password' ? credentialAction.user_id : ''}`}
              session={session}
              action={credentialAction}
              onChange={setCredentialAction}
            />
            <Button onClick={() => void accounts.refetch()} disabled={accounts.isFetching}>
              <RefreshCw size={15} /> Refresh
            </Button>
          </div>
        </div>
        {accounts.isPending && (
          <p className="connected-state" role="status">
            <LoaderCircle size={17} className="animate-spin" /> Loading accounts…
          </p>
        )}
        {accounts.error && (
          <p role="alert" className="connected-error">
            {errorText(accounts.error)} The list could not be refreshed.
          </p>
        )}
        {filtered && (
          <div className="table-scroll" role="region" aria-label="Accounts table" tabIndex={0}>
            <table>
              <thead>
                <tr>
                  <th scope="col">Account</th>
                  <th scope="col">Players</th>
                  <th scope="col">Role</th>
                  <th scope="col">Standing</th>
                  <th scope="col" className="numeric">
                    Balance
                  </th>
                  <th scope="col">
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((account) => {
                  const name = account.login ?? `#${account.user_id}`
                  return (
                    <tr key={account.user_id}>
                      <td>
                        <strong>{name}</strong>
                        <small className="block muted">#{account.user_id}</small>
                      </td>
                      <td>{account.players.map((p) => p.name).join(', ') || 'No players yet'}</td>
                      <td>{account.role}</td>
                      <td>
                        <Badge
                          tone={
                            !account.standing
                              ? 'neutral'
                              : account.standing.kind === 'banned'
                                ? 'red'
                                : account.standing?.kind === 'restricted'
                                  ? 'amber'
                                  : 'green'
                          }
                        >
                          {account.standing?.kind ?? 'Key account'}
                        </Badge>
                        {account.muted && <Badge tone="violet">Muted</Badge>}
                      </td>
                      <td className="numeric">{account.bp.toLocaleString()} BP</td>
                      <td>
                        <div className="action-group">
                          <Button
                            aria-label={`View account ${name}`}
                            onClick={() => setDetailId(account.user_id)}
                          >
                            <Eye size={15} />
                            View
                          </Button>
                          <Button
                            aria-label={`Give BP to ${name}`}
                            disabled={
                              session.role !== 'admin' ||
                              Boolean(stored.intent) ||
                              Boolean(stored.error)
                            }
                            title={session.role !== 'admin' ? 'Administrators only' : undefined}
                            onClick={() => {
                              setSelected(account)
                              setAmount('500')
                              setNotice('')
                              grant.reset()
                              setOpen(true)
                            }}
                          >
                            <Coins size={15} /> Give BP
                          </Button>
                        </div>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="connected-state">
                {query ? 'No accounts match your search.' : 'No accounts yet.'}
              </p>
            )}
          </div>
        )}
      </Panel>
      {full && (
        <>
          <ConnectedAudit />
          <ConnectedLogs />
          <ConnectedBackups session={session} />
        </>
      )}
      <ConnectedAccountDialog
        session={session}
        userId={detailId}
        onChange={setDetailId}
        onDelete={(account) => {
          setDetailId(null)
          setDeletionId(account.user_id)
        }}
        onModerate={(account) => {
          setDetailId(null)
          setModerationId(account.user_id)
        }}
        onGift={(player) => {
          setDetailId(null)
          setGiftTarget(player)
        }}
        onRevealKey={(account) => {
          setDetailId(null)
          setKeyAction({ kind: 'reveal', user_id: account.user_id })
        }}
        onResetPassword={(account) => {
          setDetailId(null)
          setCredentialAction({
            kind: 'reset_password',
            user_id: account.user_id,
            login: account.login ?? `Account #${account.user_id}`,
          })
        }}
      />
      {full && (
        <>
          <ConnectedInvites session={session} />
          <ConnectedCodes session={session} />
          <ConnectedSettings session={session} />
        </>
      )}
      <Modal
        open={open}
        onOpenChange={(value) => {
          if (!grant.isPending) setOpen(value)
        }}
        title="Give BP"
        description={`Account #${stored.intent?.data.user_id ?? selected?.user_id ?? ''}. Saved with D-Mail and an audit entry.`}
      >
        <form id="connected-grant" onSubmit={submit} className="modal-body">
          <Field id="connected-bp" label="BP amount">
            <input
              id="connected-bp"
              type="number"
              min={1}
              max={4294967295}
              step={1}
              required
              value={stored.intent?.data.amount ?? amount}
              disabled={Boolean(stored.intent) || grant.isPending}
              onChange={(e) => setAmount(e.target.value)}
            />
          </Field>
          {grant.error && (
            <p className="connected-error" role="alert">
              {stored.intent ? 'We could not confirm this grant.' : 'The grant was refused.'}{' '}
              {errorText(grant.error)}
              {stored.intent && ' Retry this same request to avoid adding BP twice.'}
            </p>
          )}
          {stored.intent && (
            <p className="field-hint">Request ID: {stored.intent.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">
            {stored.intent ? 'Request saved for safe retry' : 'Administrators only'}
          </span>
          <div className="action-group">
            <Button disabled={grant.isPending} onClick={() => setOpen(false)}>
              Close
            </Button>
            <Button
              variant="primary"
              type="submit"
              form="connected-grant"
              disabled={
                grant.isPending ||
                Boolean(stored.error) ||
                session.role !== 'admin' ||
                (!stored.intent && !validAmount)
              }
            >
              {grant.isPending && <LoaderCircle size={16} className="animate-spin" />}
              {grant.isPending ? 'Saving…' : stored.intent ? 'Retry same grant' : 'Add BP'}
            </Button>
          </div>
        </div>
      </Modal>
    </>
  )
}
