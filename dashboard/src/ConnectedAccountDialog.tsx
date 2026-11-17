import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, Pencil, Plus, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Badge, Button, Field, Modal } from './components/ui'
import {
  getGetAccountQueryKey,
  getGetStaffSessionQueryKey,
  getListAccountsQueryKey,
  useEditAccount,
  useGetAccount,
  useListCountries,
} from './api/generated/http/client'
import type { EditAccountMutationVariables } from './api/generated/http/client'
import type { AccountEditRequest, Role, StaffSession } from './api/generated/http/models'
import {
  AccountEditRequest as EditRequestSchema,
  EditAccountHeader,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

const pendingSchema = z.object({ data: EditRequestSchema, headers: EditAccountHeader })
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'
type Mode = AccountEditRequest['kind']

export function ConnectedAccountDialog({
  session,
  userId,
  onChange,
}: {
  session: StaffSession
  userId: number | null
  onChange: (id: number | null) => void
}) {
  const client = useQueryClient()
  const account = useGetAccount(userId ?? 0, { query: { enabled: userId !== null } })
  const storageKey = `yo3:pending-profile:${session.user_id}`
  const [stored, setStored] = useState<{
    intent: EditAccountMutationVariables | null
    error: string | null
  }>(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { intent: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: null }
    } catch {
      return {
        intent: null,
        error:
          'Could not restore a saved profile change. Review the audit log before making another.',
      }
    }
  })
  const [mode, setMode] = useState<Mode | null>(null)
  const countries = useListCountries({
    query: { enabled: userId !== null && mode === 'set_country' },
  })
  const [player, setPlayer] = useState(0)
  const [name, setName] = useState('')
  const [country, setCountry] = useState('')
  const [role, setRole] = useState<Role>('player')
  const [confirmedRole, setConfirmedRole] = useState(false)
  const [notice, setNotice] = useState('')
  const [failure, setFailure] = useState('')
  const [uncertain, setUncertain] = useState(false)
  const edit = useEditAccount({
    mutation: {
      onSuccess: (result, variables) => {
        try {
          sessionStorage.removeItem(storageKey)
          setStored({ intent: null, error: null })
        } catch {
          setStored((previous) => ({
            ...previous,
            error:
              'Saved, but this browser could not clear the request. Reload to confirm the same change safely.',
          }))
        }
        const roleEdit = variables.data.kind === 'set_role'
        setNotice(
          roleEdit
            ? 'Role saved. Changed accounts must sign in again.'
            : 'Profile saved. In-game changes appear after the next sign-in.',
        )
        setFailure('')
        setMode(null)
        void client.invalidateQueries({ queryKey: getGetAccountQueryKey(result.user_id) })
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        if (roleEdit && result.user_id === session.user_id) {
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
        }
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
              error: 'Could not clear the refused request. Reload before editing another profile.',
            }))
          }
        }
        if (error instanceof ApiHttpError && [401, 403].includes(error.status)) {
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
        }
      },
    },
  })
  const data =
    account.error instanceof ApiHttpError && [401, 403, 404].includes(account.error.status)
      ? undefined
      : account.data
  const admin = session.role === 'admin'
  const moderator = session.role === 'moderator'
  const canRename =
    admin || (moderator && (data?.role === 'player' || data?.user_id === session.user_id))
  const canStart =
    !edit.isPending && !stored.intent && !stored.error && Boolean(data) && !account.error
  const pending = stored.intent?.data.user_id === userId ? stored.intent : null
  const displayedCountry =
    pending?.data.kind === 'set_country' ? (pending.data.country ?? '') : country
  const displayedRole = pending?.data.kind === 'set_role' ? pending.data.role : role
  const draft = EditRequestSchema.safeParse(
    mode === 'set_role'
      ? { kind: mode, user_id: userId, role }
      : mode === 'set_country'
        ? { kind: mode, user_id: userId, country: country || null }
        : mode === 'add_player'
          ? { kind: mode, user_id: userId, name: name.trim() }
          : { kind: mode, user_id: userId, player_id: player, name: name.trim() },
  )
  const allowed = mode === 'rename_player' ? (pending ? admin || moderator : canRename) : admin
  const availableCountry =
    mode !== 'set_country' || !country || Boolean(countries.data?.codes.includes(country))

  function start(next: Mode, playerId = 0, currentName = '') {
    setPlayer(playerId)
    setName(currentName)
    setCountry(data?.country ?? '')
    setRole(data?.role ?? 'player')
    setConfirmedRole(false)
    setFailure('')
    setNotice('')
    setMode(next)
  }
  function resume() {
    if (!stored.intent) return
    onChange(stored.intent.data.user_id)
    setMode(stored.intent.data.kind)
    setFailure('')
  }
  function close() {
    if (edit.isPending) return
    setMode(null)
    setFailure('')
    setNotice('')
    onChange(null)
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (edit.isPending || stored.error || !allowed) return
    let intent = pending
    if (!intent) {
      if (
        !canStart ||
        !draft.success ||
        !availableCountry ||
        (mode === 'set_role' && !confirmedRole)
      )
        return
      intent = { data: draft.data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(intent))
      } catch {
        setStored({
          intent: null,
          error:
            'This browser could not retain the request ID. Enable session storage before editing profiles.',
        })
        return
      }
      setStored({ intent, error: null })
    }
    setFailure('')
    edit.mutate(intent)
  }

  return (
    <>
      {stored.error && (
        <p role="alert" className="connected-error">
          {stored.error}
        </p>
      )}
      {stored.intent && (
        <div className="inline-notice">
          <div>
            <strong>Unconfirmed profile change</strong>
            <p>
              Account #{stored.intent.data.user_id}. It may already be saved; retry the same request
              before making another edit.
            </p>
          </div>
          <Button onClick={resume} disabled={edit.isPending}>
            Resume profile change
          </Button>
        </div>
      )}
      <Modal
        open={userId !== null}
        onOpenChange={(value) => {
          if (!value) close()
        }}
        title={data?.login ?? `Account #${userId ?? ''}`}
        description="Account details from the server. Role changes apply immediately; in-game profile edits appear after signing in again."
        wide
      >
        <div className="modal-body connected-profile">
          <div className="table-toolbar">
            <span className="muted">
              {data
                ? `#${data.user_id} · Checked ${new Date(data.checked_at).toLocaleString()}`
                : 'Account details'}
            </span>
            <Button
              onClick={() => void account.refetch()}
              disabled={account.isFetching || edit.isPending}
            >
              <RefreshCw size={15} />
              Refresh account
            </Button>
          </div>
          {account.isPending && (
            <p role="status" className="connected-state">
              <LoaderCircle size={17} className="animate-spin" />
              Loading account…
            </p>
          )}
          {account.error && (
            <p role="alert" className="connected-error">
              Account details could not be refreshed. {errorText(account.error)}
              {account.error instanceof ApiHttpError && account.error.status === 401 && (
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
          {notice && (
            <p role="status" className="inline-notice">
              {notice}
            </p>
          )}
          {stored.error && (
            <p role="alert" className="connected-error">
              {stored.error}
            </p>
          )}
          {stored.intent && !pending && (
            <div className="inline-notice">
              <p>An edit for account #{stored.intent.data.user_id} needs confirmation first.</p>
              <Button onClick={resume}>Resume profile change</Button>
            </div>
          )}
          {pending && !mode && (
            <div className="inline-notice">
              <p>This account has an unconfirmed change.</p>
              <Button onClick={resume}>Resume this change</Button>
            </div>
          )}
          {data && !mode && (
            <>
              <dl className="profile-facts">
                <div>
                  <dt>Role</dt>
                  <dd>
                    {data.role}
                    <Button
                      onClick={() => start('set_role')}
                      disabled={!admin || !data.login || !canStart}
                      title={
                        !admin
                          ? 'Administrators only'
                          : !data.login
                            ? 'Only password accounts can have staff roles'
                            : undefined
                      }
                    >
                      Change role
                    </Button>
                  </dd>
                </div>
                <div>
                  <dt>Balance</dt>
                  <dd>{data.bp.toLocaleString()} BP</dd>
                </div>
                <div>
                  <dt>Country</dt>
                  <dd>{data.country ?? 'Automatic (game language)'}</dd>
                </div>
                <div>
                  <dt>Sign-in method</dt>
                  <dd>
                    {data.login
                      ? 'Password account'
                      : data.has_key
                        ? 'Key account'
                        : 'No sign-in configured'}
                    {data.login && data.has_key ? ' · Also has a key' : ''}
                  </dd>
                </div>
                <div>
                  <dt>Standing</dt>
                  <dd>
                    {data.standing?.kind ?? 'Not applicable'}
                    {data.standing && data.standing.kind !== 'active' && (
                      <>
                        <span className="block">{data.standing.reason}</span>
                        <span className="block muted">
                          {data.standing.until
                            ? `Until ${new Date(data.standing.until).toLocaleString()}`
                            : 'Until lifted'}
                        </span>
                      </>
                    )}
                  </dd>
                </div>
                <div>
                  <dt>Chat</dt>
                  <dd>
                    {data.muted ? (
                      <>
                        {data.muted.reason}
                        <span className="block muted">
                          {data.muted.until
                            ? `Muted until ${new Date(data.muted.until).toLocaleString()}`
                            : 'Muted until lifted'}
                        </span>
                      </>
                    ) : (
                      'Not muted'
                    )}
                  </dd>
                </div>
                <div>
                  <dt>Remembered devices</dt>
                  <dd>{data.device_count}</dd>
                </div>
                <div>
                  <dt>Password reset</dt>
                  <dd>{data.reset_pending ? 'Code pending' : 'No pending code'}</dd>
                </div>
              </dl>
              <div className="table-toolbar">
                <strong>Players ({data.players.length}/4)</strong>
                <div className="action-group">
                  <Button
                    onClick={() => start('set_country')}
                    disabled={!admin || !canStart}
                    title={!admin ? 'Administrators only' : undefined}
                  >
                    Change country
                  </Button>
                  <Button
                    onClick={() => start('add_player')}
                    disabled={!admin || !canStart || data.players.length >= 4}
                    title={
                      !admin
                        ? 'Administrators only'
                        : data.players.length >= 4
                          ? 'An account holds up to four players'
                          : undefined
                    }
                  >
                    <Plus size={15} />
                    Add player
                  </Button>
                </div>
              </div>
              <div className="table-scroll" role="region" aria-label="Account players" tabIndex={0}>
                <table>
                  <thead>
                    <tr>
                      <th scope="col">Player</th>
                      <th scope="col">Presence</th>
                      <th scope="col">Additional cards</th>
                      <th scope="col">Decks</th>
                      <th scope="col">Record</th>
                      <th scope="col">
                        <span className="sr-only">Actions</span>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.players.map((p) => (
                      <tr key={p.player_id}>
                        <td>
                          <strong>{p.name}</strong>
                          <small className="block muted">
                            #{p.player_id} · Slot {p.slot + 1} · Level {p.level} · Title #
                            {p.title_id}
                          </small>
                        </td>
                        <td>
                          <Badge tone={p.online ? 'green' : 'neutral'}>
                            {p.online == null ? 'Unknown' : p.online ? 'Online' : 'Offline'}
                          </Badge>
                        </td>
                        <td>
                          {p.additional_cards.toLocaleString()}
                          <small className="block muted">
                            {p.card_kinds} kinds · {p.owned_items} items
                          </small>
                        </td>
                        <td>
                          {p.purchased_decks} purchased
                          <small className="block muted">{p.uploaded_decks} uploaded</small>
                        </td>
                        <td>
                          {p.record
                            ? `${p.record.wins} W / ${p.record.losses} L / ${p.record.draws} D`
                            : 'Unavailable'}
                        </td>
                        <td>
                          <Button
                            aria-label={`Rename ${p.name}`}
                            onClick={() => start('rename_player', p.player_id, p.name)}
                            disabled={!canRename || !canStart}
                            title={
                              !canRename
                                ? 'Moderators cannot edit another staff account'
                                : undefined
                            }
                          >
                            <Pencil size={14} />
                            Rename
                          </Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {data.players.length === 0 && <p className="connected-state">No players yet.</p>}
              </div>
              <p className="field-hint">
                Additional card counts exclude the starter deck. Presence is a snapshot; refresh for
                the latest state.
              </p>
            </>
          )}
          {mode && (
            <form id="connected-profile-edit" onSubmit={submit}>
              <h3>
                {mode === 'add_player'
                  ? 'Add player'
                  : mode === 'rename_player'
                    ? 'Rename player'
                    : mode === 'set_role'
                      ? 'Change role'
                      : 'Change country'}
              </h3>
              {mode === 'set_role' ? (
                <>
                  <Field
                    id="profile-role"
                    label="New role"
                    hint="Players cannot use management. Moderators can help players. Administrators have full management access."
                  >
                    <select
                      id="profile-role"
                      value={displayedRole}
                      onChange={(event) => {
                        setRole(event.target.value as Role)
                        setConfirmedRole(false)
                      }}
                      disabled={edit.isPending || Boolean(pending)}
                    >
                      <option value="player">Player</option>
                      <option value="moderator">Moderator</option>
                      <option value="admin">Administrator</option>
                    </select>
                  </Field>
                  <p className="field-hint">
                    A changed role signs this account out of both management interfaces. It does not
                    disconnect its game.
                  </p>
                  {userId === session.user_id && displayedRole !== 'admin' && (
                    <p role="alert" className="connected-error">
                      Changing your own role signs you out. You will lose administrator access.
                    </p>
                  )}
                  {pending && !admin && (
                    <p role="alert" className="connected-error">
                      Only an administrator can confirm this request. Ask the host to check the
                      account's current role before making another change.
                    </p>
                  )}
                  <label className="profile-role-confirm">
                    <input
                      type="checkbox"
                      checked={Boolean(pending) || confirmedRole}
                      onChange={(event) => setConfirmedRole(event.target.checked)}
                      disabled={edit.isPending || Boolean(pending)}
                    />
                    I confirm this role change for account #{userId}.
                  </label>
                </>
              ) : mode === 'set_country' ? (
                <>
                  <Field
                    id="profile-country"
                    label="Country"
                    hint="Automatic uses the game's language. Available flags come from the original game files."
                  >
                    <select
                      id="profile-country"
                      value={displayedCountry}
                      onChange={(e) => setCountry(e.target.value)}
                      disabled={edit.isPending || Boolean(pending)}
                    >
                      <option value="">Automatic (game language)</option>
                      {displayedCountry && !countries.data?.codes.includes(displayedCountry) && (
                        <option value={displayedCountry}>
                          {displayedCountry} ({pending ? 'pending change' : 'current'})
                        </option>
                      )}
                      {countries.data?.codes.map((code) => (
                        <option key={code} value={code}>
                          {code}
                        </option>
                      ))}
                    </select>
                  </Field>
                  {countries.isFetching && <p role="status">Loading country choices…</p>}
                  {countries.error && (
                    <div role="alert" className="connected-error">
                      {errorText(countries.error)}
                      <Button
                        onClick={() => void countries.refetch()}
                        disabled={countries.isFetching}
                      >
                        Retry country choices
                      </Button>
                    </div>
                  )}
                </>
              ) : (
                <Field
                  id="profile-name"
                  label="Player name"
                  hint="1–16 printable characters. Names are unique across all accounts."
                >
                  <input
                    id="profile-name"
                    value={
                      pending &&
                      (pending.data.kind === 'add_player' || pending.data.kind === 'rename_player')
                        ? pending.data.name
                        : name
                    }
                    maxLength={16}
                    required
                    onChange={(e) => setName(e.target.value)}
                    disabled={edit.isPending || Boolean(pending)}
                  />
                </Field>
              )}
              {failure && (
                <div role="alert" className="connected-error">
                  <p>
                    {uncertain
                      ? 'We could not confirm this profile change.'
                      : 'The profile change was refused.'}
                  </p>
                  <p>{failure}</p>
                  {pending && <p>Retry the same request to avoid applying it twice.</p>}
                </div>
              )}
              {pending && (
                <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
              )}
            </form>
          )}
        </div>
        <div className="modal-actions">
          <span className="muted">
            {pending
              ? 'Request saved for safe retry'
              : mode
                ? 'Saved changes are audited'
                : 'Credentials are never shown'}
          </span>
          <div className="action-group">
            {mode && !pending && (
              <Button
                onClick={() => {
                  setMode(null)
                  setFailure('')
                }}
                disabled={edit.isPending}
              >
                Back
              </Button>
            )}
            {(!mode || pending) && (
              <Button onClick={close} disabled={edit.isPending}>
                Close
              </Button>
            )}
            {mode && (
              <Button
                variant="primary"
                type="submit"
                form="connected-profile-edit"
                disabled={
                  edit.isPending ||
                  !allowed ||
                  Boolean(stored.error) ||
                  (!pending &&
                    (!canStart ||
                      !draft.success ||
                      !availableCountry ||
                      (mode === 'set_role' && !confirmedRole)))
                }
              >
                {edit.isPending && <LoaderCircle size={16} className="animate-spin" />}
                {edit.isPending
                  ? 'Saving…'
                  : pending
                    ? 'Retry same change'
                    : mode === 'add_player'
                      ? 'Add player'
                      : mode === 'rename_player'
                        ? 'Save name'
                        : mode === 'set_role'
                          ? 'Save role'
                          : 'Save country'}
              </Button>
            )}
          </div>
        </div>
      </Modal>
    </>
  )
}
