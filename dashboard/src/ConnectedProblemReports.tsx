import { useState } from 'react'
import type { FormEvent } from 'react'
import { Download, LoaderCircle, RefreshCw } from 'lucide-react'
import { Input } from './components/form-controls'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import { useGetProblemReport, useListProblemReports } from './api/generated/http/client'
import type { ProblemReportRow } from './api/generated/http/models'
import { ApiHttpError } from './api/transport'
import { downloadText } from './download'

const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'
const when = (at: number) => new Date(at * 1000).toLocaleString()

function Report({ id }: { id: string }) {
  const report = useGetProblemReport(id)
  if (report.isPending)
    return (
      <p role="status" className="connected-state">
        <LoaderCircle size={17} className="animate-spin" />
        Loading the report…
      </p>
    )
  if (report.error)
    return (
      <p role="alert" className="connected-error">
        The report could not be loaded. {errorText(report.error)}
      </p>
    )
  const r = report.data
  return (
    <>
      <p className="muted">
        {r.account} · {r.kind} · launcher {r.launcher} · {when(r.at)}
      </p>
      <p>{r.summary}</p>
      <div className="action-group">
        <Button onClick={() => downloadText(`${r.id}.txt`, r.text)}>
          <Download size={15} />
          Download
        </Button>
      </div>
      <pre
        className="connected-log-output"
        tabIndex={0}
        aria-label="The server's lines, then the launcher's log"
      >
        {[...r.server_log, '', '== The launcher’s log ==', r.logs].join('\n')}
      </pre>
    </>
  )
}

export function ConnectedProblemReports() {
  const [who, setWho] = useState('')
  const [filter, setFilter] = useState('')
  const [open, setOpen] = useState<ProblemReportRow | null>(null)
  const reports = useListProblemReports(filter ? { who: filter } : {})

  function apply(event: FormEvent) {
    event.preventDefault()
    setFilter(who.trim())
  }

  return (
    <section aria-label="Problem reports" className="connected-reports">
      <Panel
        title="Problem reports"
        description="Logs players' launchers sent when a game broke, or with Report a problem. Players quote the ID on GitHub."
      >
        <form className="table-toolbar" onSubmit={apply}>
          <Field id="problem-who" label="Account">
            <Input
              id="problem-who"
              value={who}
              placeholder="User ID or player name"
              onChange={(event) => setWho(event.target.value)}
            />
          </Field>
          <Button type="submit">Show</Button>
          <span className="muted">
            {reports.data ? `checked ${new Date(reports.data.checked_at).toLocaleString()}` : ''}
          </span>
          <Button
            type="button"
            onClick={() => void reports.refetch()}
            disabled={reports.isFetching}
          >
            <RefreshCw size={15} />
            Refresh
          </Button>
        </form>
        {reports.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading problem reports…
          </p>
        )}
        {reports.error && (
          <p role="alert" className="connected-error">
            The problem reports could not be loaded. {errorText(reports.error)}
          </p>
        )}
        {reports.data && (
          <>
            <div
              className="table-scroll"
              role="region"
              aria-label="Problem reports table"
              tabIndex={0}
            >
              <table className="responsive-table">
                <thead>
                  <tr>
                    <th scope="col">Time</th>
                    <th scope="col">Account</th>
                    <th scope="col">Kind</th>
                    <th scope="col">Summary</th>
                    <th scope="col">ID</th>
                  </tr>
                </thead>
                <tbody>
                  {reports.data.reports.map((r) => (
                    <tr key={r.id}>
                      <td data-label="Time">{when(r.at)}</td>
                      <td data-label="Account">{r.account}</td>
                      <td data-label="Kind">
                        <Badge tone="neutral">{r.kind}</Badge>
                      </td>
                      <td data-label="Summary">{r.summary}</td>
                      <td className="row-heading numeric" data-label="ID">
                        <Button aria-label={`Open report ${r.id}`} onClick={() => setOpen(r)}>
                          {r.id}
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {reports.data.reports.length === 0 && (
              <p className="connected-state">No problem reports.</p>
            )}
          </>
        )}
      </Panel>
      <Modal
        open={open !== null}
        onOpenChange={(value) => {
          if (!value) setOpen(null)
        }}
        title={open ? `Problem report ${open.id}` : 'Problem report'}
        description="Read only. Players' passwords and keys are scrubbed before the launcher sends a log."
        wide
      >
        {open && <Report id={open.id} />}
      </Modal>
    </section>
  )
}
