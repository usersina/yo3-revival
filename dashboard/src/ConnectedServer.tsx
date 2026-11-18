import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Badge, Button, Modal, Panel } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  useGetServerStatus,
  useStopServer,
} from './api/generated/http/client'
import type { StopServerMutationVariables } from './api/generated/http/client'
import type { StaffSession, StopResponse } from './api/generated/http/models'
import { StopRequest, StopServerHeader } from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'
const pendingSchema = z.object({ data: StopRequest, headers: StopServerHeader })
function memoryLabel(raw: string | null | undefined) {
  if (raw == null) return 'Not available'
  try {
    const bytes = BigInt(raw)
    return `${bytes / 1048576n}.${((bytes % 1048576n) * 10n) / 1048576n} MiB`
  } catch {
    return 'Not available'
  }
}
export function ConnectedServer({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const status = useGetServerStatus({ query: { refetchInterval: 10000 } })
  const key = `yo3:pending-stop:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(key)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error:
          'Could not restore a stop request. Check the audit log before requesting another stop.',
      }
    }
  })
  const [pending, setPending] = useState<StopServerMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [open, setOpen] = useState(false)
  const [confirmed, setConfirmed] = useState(false)
  const [result, setResult] = useState<StopResponse | null>(null)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  function clearIntent() {
    try {
      sessionStorage.removeItem(key)
      setPending(null)
      return true
    } catch {
      setStorageError('Could not clear this stop request. No new stop was sent.')
      return false
    }
  }
  const mutation = useStopServer({
    mutation: {
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        if (value.stop === 'completed') clearIntent()
        void status.refetch()
      },
      onError: (error) => {
        setFailure(
          error instanceof ApiHttpError
            ? error.message
            : 'The stop could not be confirmed. Retry this same request when the server is reachable; it cannot stop a different server run.',
        )
        setRefused(
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status),
        )
        if (error instanceof ApiHttpError && [401, 403].includes(error.status))
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
    },
  })
  function close() {
    if (mutation.isPending || (result && !clearIntent())) return
    setOpen(false)
    setResult(null)
    setFailure('')
    setRefused(false)
    setConfirmed(false)
    mutation.reset()
  }
  function edit() {
    if (!refused || !clearIntent()) return
    setRefused(false)
    setFailure('')
    setConfirmed(false)
    void status.refetch()
    mutation.reset()
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (mutation.isPending || storageError || refused || result || session.role !== 'admin') return
    let variables = pending
    if (!variables) {
      if (!status.data?.can_stop || status.isFetching || status.error || !confirmed) return
      variables = {
        data: { run_id: status.data.run_id },
        headers: { 'Idempotency-Key': crypto.randomUUID() },
      }
      try {
        sessionStorage.setItem(key, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this stop request safely. No request was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  return (
    <section aria-label="Live server">
      <Panel
        className="connected-panel"
        title="Live server"
        description="Checked from the running process. Presence and duels reset on restart."
        action={
          <Button
            aria-label="Refresh server status"
            onClick={() => void status.refetch()}
            disabled={status.isFetching}
          >
            <RefreshCw size={15} />
            <span>Refresh</span>
          </Button>
        }
      >
        {status.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={16} className="spin" /> Reading server status…
          </p>
        )}
        {status.error && (
          <p role="alert" className="connected-error">
            The server status could not be refreshed. Check that the server is reachable, then
            refresh. Any figures below are from the last successful check.
          </p>
        )}
        {status.data && (
          <>
            <dl className="profile-facts">
              <div>
                <dt>Version</dt>
                <dd>{status.data.version}</dd>
              </div>
              <div>
                <dt>Online players</dt>
                <dd>{status.data.online}</dd>
              </div>
              <div>
                <dt>Running duels</dt>
                <dd>{status.data.duels}</dd>
              </div>
              <div>
                <dt>Saved accounts / players</dt>
                <dd>
                  {status.data.accounts} / {status.data.players}
                </dd>
              </div>
              <div>
                <dt>Uptime</dt>
                <dd>
                  {status.data.uptime_seconds == null
                    ? 'Not available'
                    : `${Math.floor(status.data.uptime_seconds / 60)} min`}
                </dd>
              </div>
              <div>
                <dt>Rules engines</dt>
                <dd>{status.data.engines}</dd>
              </div>
              <div>
                <dt>Server memory</dt>
                <dd>{memoryLabel(status.data.memory_bytes)}</dd>
              </div>
              <div>
                <dt>Latest backup</dt>
                <dd>
                  {status.data.backup.kind === 'available' ? (
                    new Date(status.data.backup.at).toLocaleString()
                  ) : status.data.backup.kind === 'none' ? (
                    <Badge tone="amber">None yet</Badge>
                  ) : (
                    <Badge tone="red">Unavailable</Badge>
                  )}
                </dd>
              </div>
              <div>
                <dt>Last checked</dt>
                <dd>{new Date(status.data.checked_at).toLocaleTimeString()}</dd>
              </div>
            </dl>
            {status.data.backup.kind === 'unavailable' && (
              <p role="alert" className="connected-error">
                Backup check failed: {status.data.backup.message}{' '}
                <Link to="/backups">Open backups</Link>
              </p>
            )}
            {status.data.backup.kind === 'none' && (
              <p className="inline-notice">
                No database snapshot yet.{' '}
                <Link to="/backups">
                  {session.role === 'admin' ? 'Create a backup' : 'View backups'}
                </Link>
                {session.role === 'admin'
                  ? ' and keep a private copy off this machine.'
                  : ' and ask an administrator to create a snapshot.'}
              </p>
            )}
            {status.data.backup.kind === 'available' &&
              Date.parse(status.data.checked_at) - Date.parse(status.data.backup.at) > 7200000 && (
                <p role="alert" className="connected-error">
                  The latest snapshot is over two hours old.{' '}
                  <Link to="/backups">Check backups</Link> and inspect the server logs.
                </p>
              )}
            {!status.data.persistence_healthy && (
              <p role="alert" className="connected-error">
                Saving has failed. New sign-ins are refused until a save succeeds. Check disk space
                and the server logs.
              </p>
            )}
            {status.data.engine_problem && (
              <p role="alert" className="connected-error">
                Engine unavailable: {status.data.engine_problem}
              </p>
            )}
          </>
        )}
        {status.data && (
          <div className="connected-online">
            <h3>Online now</h3>
            {status.data.online_players.length ? (
              <div className="table-scroll" role="region" aria-label="Online players" tabIndex={0}>
                <table className="responsive-table">
                  <thead>
                    <tr>
                      <th>Player</th>
                      <th>Activity</th>
                    </tr>
                  </thead>
                  <tbody>
                    {status.data.online_players.map((player) => (
                      <tr key={player.player_id}>
                        <td className="row-heading" data-label="Player">
                          {player.user_id == null ? (
                            player.name
                          ) : (
                            <Link to={`/accounts/${player.user_id}`}>{player.name}</Link>
                          )}
                        </td>
                        <td data-label="Activity">
                          {player.duelling ? 'Duelling' : 'In the game'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="muted">Nobody is online.</p>
            )}
          </div>
        )}
        {storageError && !open && (
          <p role="alert" className="connected-error">
            {storageError}
          </p>
        )}
        {session.role === 'admin' && (
          <Button
            variant="danger"
            disabled={
              !pending &&
              (!status.data?.can_stop ||
                status.isFetching ||
                Boolean(status.error) ||
                Boolean(storageError))
            }
            title={
              status.data && !status.data.can_stop
                ? 'This API does not own a server process.'
                : undefined
            }
            onClick={() => setOpen(true)}
          >
            {pending ? 'Resume stop request' : 'Stop server…'}
          </Button>
        )}
        <Modal
          open={open}
          onOpenChange={(value) => {
            if (!value) close()
          }}
          title="Stop server"
          description="Disconnects all games and requests a graceful shutdown with final saves and a backup."
        >
          <form id="connected-stop-server" onSubmit={submit} className="modal-body">
            {result ? (
              <>
                <p role="status" className="inline-notice notice-success">
                  {result.result.message}
                </p>
                <p>
                  This confirms the signal was requested, not that the process has finished
                  stopping.
                </p>
                {result.stop !== 'completed' && (
                  <p role="alert" className="inline-notice notice-warning">
                    The signal outcome is unknown. Check the server process and logs. This request
                    will never signal another server run.
                  </p>
                )}
              </>
            ) : pending ? (
              <p className="inline-notice notice-warning">
                This request is bound to server run {pending.data.run_id.slice(0, 12)}. Retry it to
                check what happened; a restarted server will not be stopped.
              </p>
            ) : (
              <label className="profile-role-confirm">
                <input
                  type="checkbox"
                  checked={confirmed}
                  onChange={(event) => setConfirmed(event.target.checked)}
                />
                I understand all connected games will disconnect.
              </label>
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
            <Button onClick={close} disabled={mutation.isPending}>
              {result ? 'Done' : 'Cancel'}
            </Button>
            {result && pending && (
              <Button onClick={() => mutation.mutate(pending)} disabled={mutation.isPending}>
                Check outcome
              </Button>
            )}
            {refused && pending && <Button onClick={edit}>Review new request</Button>}
            {!result && (
              <Button
                form="connected-stop-server"
                type="submit"
                variant="danger"
                disabled={
                  mutation.isPending ||
                  Boolean(storageError) ||
                  refused ||
                  session.role !== 'admin' ||
                  (!pending &&
                    (!confirmed ||
                      !status.data?.can_stop ||
                      Boolean(status.error) ||
                      status.isFetching))
                }
              >
                {mutation.isPending ? (
                  <>
                    <LoaderCircle size={16} className="spin" /> Requesting stop…
                  </>
                ) : pending ? (
                  'Retry same stop'
                ) : (
                  'Stop server'
                )}
              </Button>
            )}
          </div>
        </Modal>
      </Panel>
    </section>
  )
}
