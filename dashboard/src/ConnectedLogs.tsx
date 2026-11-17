import { useState } from 'react'
import { Copy, LoaderCircle, RefreshCw } from 'lucide-react'
import { Button, Field, Panel } from './components/ui'
import { useGetServerLogs } from './api/generated/http/client'
import { ApiHttpError } from './api/transport'
export function ConnectedLogs() {
  const [problems, setProblems] = useState(false)
  const [limit, setLimit] = useState(200)
  const [automatic, setAutomatic] = useState(false)
  const [copyState, setCopyState] = useState('')
  const logs = useGetServerLogs(
    { limit, problems },
    { query: { refetchInterval: automatic ? 5000 : false } },
  )
  async function copy() {
    try {
      await navigator.clipboard.writeText(
        logs.data?.lines.map((line) => line.text).join('\n') ?? '',
      )
      setCopyState('Visible log lines copied.')
    } catch {
      setCopyState('Could not copy logs. Select the text below and copy it manually.')
    }
  }
  return (
    <section aria-label="Server logs">
      <Panel
        title="Server logs"
        description="Recent lines from this process, newest first. Verbose recordings stay in the server’s diagnostic files."
        action={
          <div className="action-group">
            <Button disabled={!logs.data?.lines.length} onClick={() => void copy()}>
              <Copy size={15} /> Copy visible logs
            </Button>
            <Button
              aria-label="Refresh server logs"
              disabled={logs.isFetching}
              onClick={() => void logs.refetch()}
            >
              <RefreshCw size={15} /> Refresh
            </Button>
          </div>
        }
      >
        <div className="table-toolbar">
          <label className="profile-role-confirm">
            <input
              type="checkbox"
              checked={problems}
              onChange={(event) => setProblems(event.target.checked)}
            />{' '}
            Warnings and errors only
          </label>
          <label className="profile-role-confirm">
            <input
              type="checkbox"
              checked={automatic}
              onChange={(event) => setAutomatic(event.target.checked)}
            />{' '}
            Refresh every 5 seconds
          </label>
          <Field id="logs-limit" label="Maximum lines">
            <select
              id="logs-limit"
              value={limit}
              onChange={(event) => setLimit(Number(event.target.value))}
            >
              {[50, 100, 200, 500].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </Field>
        </div>
        {logs.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={16} className="spin" /> Loading server logs…
          </p>
        )}
        {logs.error && (
          <p role="alert" className="connected-error">
            {logs.error instanceof ApiHttpError
              ? logs.error.message
              : 'The server logs could not be refreshed. Try again when the server is reachable.'}{' '}
            {logs.data ? 'Showing the last successfully loaded lines.' : ''}
          </p>
        )}
        {copyState && (
          <p role="status" className="inline-notice">
            {copyState}
          </p>
        )}
        {logs.data && (
          <>
            <p className="connected-state">
              Last checked {new Date(logs.data.checked_at).toLocaleTimeString()} · Up to{' '}
              {logs.data.retention_lines.toLocaleString()} lines retained in memory. Restarting
              clears this view.
            </p>
            {logs.data.lines.length ? (
              <pre
                className="connected-log-output"
                role="log"
                aria-live="off"
                tabIndex={0}
                aria-label="Recent server log lines"
              >
                {logs.data.lines.map((line) => line.text).join('\n')}
              </pre>
            ) : (
              !logs.error && (
                <p className="connected-state">
                  {problems
                    ? 'No warnings or errors in the retained logs.'
                    : 'No log lines captured by this process yet.'}
                </p>
              )
            )}
          </>
        )}
      </Panel>
    </section>
  )
}
