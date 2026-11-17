import { useState } from 'react'
import type { FormEvent } from 'react'
import { LoaderCircle, RefreshCw } from 'lucide-react'
import { keepPreviousData } from '@tanstack/react-query'
import { Button, Field, Panel, SearchField } from './components/ui'
import { useGetAuditHistory } from './api/generated/http/client'
import { ApiHttpError } from './api/transport'
export function ConnectedAudit() {
  const [draft, setDraft] = useState('')
  const [search, setSearch] = useState('')
  const [limit, setLimit] = useState(50)
  const [cursors, setCursors] = useState<string[]>([])
  const history = useGetAuditHistory(
    { limit, q: search, ...(cursors.length ? { before: cursors.at(-1) } : {}) },
    { query: { placeholderData: keepPreviousData } },
  )
  function filter(event: FormEvent) {
    event.preventDefault()
    setCursors([])
    setSearch(draft.trim())
  }
  return (
    <section aria-label="Audit log">
      <Panel
        title="Audit log"
        description="Saved administrative changes. Newest first; old entries follow the configured retention policy."
        action={
          <Button
            aria-label="Refresh audit history"
            disabled={history.isFetching}
            onClick={() => void history.refetch()}
          >
            <RefreshCw size={15} /> Refresh
          </Button>
        }
      >
        <form onSubmit={filter} className="table-toolbar">
          <SearchField
            label="Search audit history"
            placeholder="Actor, action, target or request ID…"
            maxLength={80}
            value={draft}
            onChange={setDraft}
          />
          <Button type="submit" disabled={history.isFetching || draft.length > 80}>
            Search
          </Button>
          <Field id="audit-limit" label="Entries per page">
            <select
              id="audit-limit"
              value={limit}
              onChange={(event) => {
                setLimit(Number(event.target.value))
                setCursors([])
              }}
            >
              {[5, 25, 50, 100, 200].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </Field>
        </form>
        {history.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={16} className="spin" /> Loading audit history…
          </p>
        )}
        {history.error && (
          <p role="alert" className="connected-error">
            {history.error instanceof ApiHttpError
              ? history.error.message
              : 'The audit history could not be refreshed. Try again when the server is reachable.'}{' '}
            {history.data ? 'Showing the last successfully loaded page.' : ''}
          </p>
        )}
        {history.data && (
          <>
            <p className="connected-state">
              Last checked {new Date(history.data.checked_at).toLocaleTimeString()}
              {history.isFetching ? ' · Refreshing…' : ''}
            </p>
            <div className="table-scroll" role="region" aria-label="Audit entries" tabIndex={0}>
              <table>
                <thead>
                  <tr>
                    <th>When</th>
                    <th>Actor</th>
                    <th>Action</th>
                    <th>Target</th>
                    <th>Details</th>
                  </tr>
                </thead>
                <tbody>
                  {history.data.entries.map((entry) => (
                    <tr key={entry.id} data-audit-id={entry.id}>
                      <td>{new Date(entry.at).toLocaleString()}</td>
                      <td>{entry.actor}</td>
                      <td>{entry.operation}</td>
                      <td>{entry.target}</td>
                      <td>
                        <details>
                          <summary>View entry</summary>
                          <div className="audit-entry-detail">
                            <p>
                              Source: {entry.origin} · Entry {entry.id}
                            </p>
                            {(['before', 'after', 'detail'] as const)
                              .filter((key) => entry[key] != null)
                              .map((key) => (
                                <div key={key}>
                                  <strong>{key}</strong>
                                  <pre>{entry[key]}</pre>
                                </div>
                              ))}
                          </div>
                        </details>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {!history.error && !history.data.entries.length && (
              <p className="connected-state">
                {search ? 'No audit entries match your search.' : 'No audit entries yet.'}
              </p>
            )}
            <div className="table-toolbar">
              <Button
                disabled={!cursors.length || history.isFetching}
                onClick={() => setCursors((previous) => previous.slice(0, -1))}
              >
                Newer entries
              </Button>
              <Button
                disabled={!history.data.next_before || history.isFetching}
                onClick={() => {
                  if (history.data?.next_before)
                    setCursors((previous) => [...previous, history.data.next_before!])
                }}
              >
                Older entries
              </Button>
              {cursors.length > 0 && (
                <Button disabled={history.isFetching} onClick={() => setCursors([])}>
                  Back to newest
                </Button>
              )}
            </div>
          </>
        )}
      </Panel>
    </section>
  )
}
