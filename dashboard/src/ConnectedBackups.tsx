import { useState } from 'react'
import { DatabaseBackup, Download, LoaderCircle, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Button, Modal, Panel } from './components/ui'
import { useCreateBackup, useDownloadBackup, useListBackups } from './api/generated/http/client'
import type {
  CreateBackupMutationVariables,
  DownloadBackupMutationVariables,
} from './api/generated/http/client'
import type { CreateBackupResponse, StaffSession } from './api/generated/http/models'
import {
  CreateBackupHeader,
  CreateBackupRequest,
  DownloadBackupHeader,
  DownloadBackupRequest,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'
const creationSchema = z.object({ data: CreateBackupRequest, headers: CreateBackupHeader })
const downloadSchema = z.object({ data: DownloadBackupRequest, headers: DownloadBackupHeader })
function readPending<T>(key: string, schema: z.ZodType<T>): { value: T | null; error: string } {
  try {
    const raw = sessionStorage.getItem(key)
    return { value: raw ? schema.parse(JSON.parse(raw)) : null, error: '' }
  } catch {
    return {
      value: null,
      error:
        'Could not restore the previous backup request. Check the audit log before starting another.',
    }
  }
}
function errorText(error: unknown) {
  return error instanceof ApiHttpError
    ? error.message
    : 'The response was lost. Retry the same request when the server is reachable.'
}
export function ConnectedBackups({ session }: { session: StaffSession }) {
  const list = useListBackups()
  const createKey = `yo3:pending-backup:${session.user_id}`
  const downloadKey = `yo3:pending-backup-download:${session.user_id}`
  const [initial] = useState(() => ({
    create: readPending(createKey, creationSchema),
    download: readPending(downloadKey, downloadSchema),
  }))
  const [creation, setCreation] = useState<CreateBackupMutationVariables | null>(
    initial.create.value,
  )
  const [download, setDownload] = useState<DownloadBackupMutationVariables | null>(
    initial.download.value,
  )
  const [storageError, setStorageError] = useState(initial.create.error || initial.download.error)
  const [outcome, setOutcome] = useState<CreateBackupResponse | null>(null)
  const [notice, setNotice] = useState('')
  const [selected, setSelected] = useState<string | null>(null)
  const [confirmed, setConfirmed] = useState(false)
  function clear(kind: 'create' | 'download') {
    try {
      sessionStorage.removeItem(kind === 'create' ? createKey : downloadKey)
      if (kind === 'create') setCreation(null)
      else setDownload(null)
      return true
    } catch {
      setStorageError('Could not clear the retained request. No new request will be sent.')
      return false
    }
  }
  const create = useCreateBackup({
    mutation: {
      onSuccess: (value) => {
        setOutcome(value)
        if (['completed', 'failed'].includes(value.backup)) clear('create')
        void list.refetch()
      },
    },
  })
  const save = useDownloadBackup({
    mutation: {
      onSuccess: (blob, variables) => {
        const url = URL.createObjectURL(blob)
        const link = document.createElement('a')
        link.href = url
        link.download = `yo3-backup-${variables.data.name}.db`
        link.click()
        window.setTimeout(() => URL.revokeObjectURL(url), 60000)
        clear('download')
        setSelected(null)
        setConfirmed(false)
        setNotice('Download started. Keep this account-data backup private.')
      },
    },
  })
  function createNow() {
    if (create.isPending || storageError || session.role !== 'admin') return
    let variables = creation
    if (!variables) {
      variables = { data: {}, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        sessionStorage.setItem(createKey, JSON.stringify(variables))
        setCreation(variables)
      } catch {
        setStorageError('Could not retain this request safely. No backup was requested.')
        return
      }
    }
    setOutcome(null)
    create.mutate(variables)
  }
  function downloadNow() {
    if (!confirmed || !selected || save.isPending || storageError || session.role !== 'admin')
      return
    let variables = download
    if (!variables) {
      variables = { data: { name: selected }, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      try {
        sessionStorage.setItem(downloadKey, JSON.stringify(variables))
        setDownload(variables)
      } catch {
        setStorageError('Could not retain this download request safely. No request was sent.')
        return
      }
    }
    save.mutate(variables)
  }
  return (
    <section aria-label="Database backups">
      <Panel
        className="connected-panel"
        title="Database backups"
        description="Checked snapshots on this server. Copy them off the host to protect against losing the machine."
        action={
          <div className="action-group">
            <Button
              aria-label="Refresh backups"
              disabled={list.isFetching}
              onClick={() => void list.refetch()}
            >
              <RefreshCw size={15} /> Refresh
            </Button>
            {session.role === 'admin' && (
              <Button onClick={createNow} disabled={create.isPending || Boolean(storageError)}>
                <DatabaseBackup size={15} />{' '}
                {create.isPending
                  ? 'Creating backup…'
                  : creation
                    ? 'Check backup outcome'
                    : 'Create backup'}
              </Button>
            )}
          </div>
        }
      >
        {list.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={16} className="spin" /> Loading backups…
          </p>
        )}
        {list.error && (
          <p role="alert" className="connected-error">
            {list.error instanceof ApiHttpError
              ? list.error.message
              : 'The backup list could not be refreshed. Try again when the server is reachable.'}{' '}
            {list.data && 'Showing the last successfully loaded list.'}
          </p>
        )}
        {list.data && !list.data.downloads_allowed && (
          <p className="connected-state">
            Download copies through SSH. Public backup downloads are disabled.
          </p>
        )}
        {storageError && (
          <p role="alert" className="connected-error">
            {storageError}
          </p>
        )}
        {create.error && (
          <p role="alert" className="connected-error">
            {errorText(create.error)} The original request is retained; checking its outcome will
            not create another backup.
          </p>
        )}
        {creation &&
          create.error instanceof ApiHttpError &&
          [400, 404, 409, 413, 415].includes(create.error.status) && (
            <Button
              onClick={() => {
                if (clear('create')) create.reset()
              }}
            >
              Dismiss refused backup request
            </Button>
          )}
        {outcome && (
          <div
            className={outcome.backup === 'completed' ? 'inline-notice' : 'connected-error'}
            role={outcome.backup === 'completed' ? 'status' : 'alert'}
          >
            {outcome.backup === 'completed'
              ? outcome.available
                ? 'Backup created.'
                : 'This backup completed but is no longer retained on this server.'
              : outcome.backup === 'failed'
                ? 'Backup failed. Check server logs before creating a new backup.'
                : 'Backup outcome is unconfirmed. Check the saved outcome; do not start a replacement request.'}
            {creation && outcome.backup === 'interrupted' && (
              <Button
                onClick={() => {
                  if (clear('create')) setOutcome(null)
                }}
              >
                Acknowledge interrupted request
              </Button>
            )}
          </div>
        )}
        {download && (
          <p className="inline-notice">
            Unconfirmed download: {download.data.name}.{' '}
            <Button
              onClick={() => {
                setSelected(download.data.name)
                setConfirmed(false)
              }}
            >
              Resume download
            </Button>
          </p>
        )}
        {notice && (
          <p role="status" className="inline-notice">
            {notice}
          </p>
        )}
        {list.data && (
          <>
            <p className="connected-state">
              Last checked {new Date(list.data.checked_at).toLocaleTimeString()}. Restore is an
              offline host command with the server stopped.
            </p>
            {list.data.backups.length ? (
              <div
                className="table-scroll"
                role="region"
                aria-label="Available database backups"
                tabIndex={0}
              >
                <table className="responsive-table">
                  <thead>
                    <tr>
                      <th>Snapshot</th>
                      <th>Created</th>
                      <th>Size</th>
                      {session.role === 'admin' && list.data.downloads_allowed && <th>Download</th>}
                    </tr>
                  </thead>
                  <tbody>
                    {list.data.backups.map((row) => (
                      <tr key={row.name}>
                        <td className="row-heading" data-label="Snapshot">
                          <code>{row.name}</code>
                        </td>
                        <td data-label="Created">
                          {row.at ? new Date(row.at).toLocaleString() : 'Unknown'}
                        </td>
                        <td data-label="Size">{row.bytes} bytes</td>
                        {session.role === 'admin' && list.data.downloads_allowed && (
                          <td className="row-actions">
                            <Button
                              aria-label={`Download ${row.name}`}
                              disabled={
                                Boolean(download) || save.isPending || Boolean(storageError)
                              }
                              onClick={() => {
                                setSelected(row.name)
                                setConfirmed(false)
                                save.reset()
                              }}
                            >
                              <Download size={15} /> Download
                            </Button>
                          </td>
                        )}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              !list.error && <p className="connected-state">No backups yet.</p>
            )}
          </>
        )}
      </Panel>
      <Modal
        open={selected !== null}
        onOpenChange={(open) => {
          if (!open && !save.isPending) {
            setSelected(null)
            setConfirmed(false)
          }
        }}
        title="Download database backup"
        description="Contains account data, password hashes and audit history. Store it privately; never attach it to a public bug report."
      >
        <div className="modal-body">
          <p className="backup-snapshot">Snapshot: {selected}</p>
          <label className="profile-role-confirm">
            <input
              type="checkbox"
              checked={confirmed}
              disabled={save.isPending}
              onChange={(event) => setConfirmed(event.target.checked)}
            />{' '}
            I will keep this backup private.
          </label>
          {save.error && (
            <p role="alert" className="connected-error">
              {errorText(save.error)}
            </p>
          )}
        </div>
        <div className="modal-actions">
          <Button disabled={save.isPending} onClick={() => setSelected(null)}>
            Cancel
          </Button>
          {download &&
            save.error instanceof ApiHttpError &&
            [400, 404, 409].includes(save.error.status) && (
              <Button
                onClick={() => {
                  if (clear('download')) {
                    setSelected(null)
                    save.reset()
                    void list.refetch()
                  }
                }}
              >
                Dismiss unavailable download
              </Button>
            )}
          <Button
            variant="primary"
            disabled={!confirmed || save.isPending || Boolean(storageError)}
            onClick={downloadNow}
          >
            {save.isPending
              ? 'Preparing download…'
              : download
                ? 'Retry same download'
                : 'Download backup'}
          </Button>
        </div>
      </Modal>
    </section>
  )
}
