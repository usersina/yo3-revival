import { Input } from './components/form-controls'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, RefreshCw } from 'lucide-react'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import { SelectControl, SelectOption } from './components/form-controls'
import {
  getListReportsQueryKey,
  useHandleReport,
  useListReports,
} from './api/generated/http/client'
import type { ReportView, StaffSession, Status } from './api/generated/http/models'
import { ApiHttpError } from './api/transport'

const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'
type Filter = Status | 'all'
const tone = { open: 'amber', handled: 'green', dismissed: 'neutral' } as const
const label = { open: 'Open', handled: 'Handled', dismissed: 'Dismissed' } as const

function Player({ name, user }: { name: string; user?: number | null }) {
  return user == null ? (
    <span>{name}</span>
  ) : (
    <Link to={`/accounts/${user}`} aria-label={`View account of ${name}`}>
      {name}
    </Link>
  )
}

export function ConnectedReports({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const [filter, setFilter] = useState<Filter>('open')
  const reports = useListReports(filter === 'all' ? {} : { status: filter })
  const [target, setTarget] = useState<{
    report: ReportView
    status: 'handled' | 'dismissed'
  } | null>(null)
  const [note, setNote] = useState('')
  const [requestId, setRequestId] = useState('')
  const [notice, setNotice] = useState('')
  const allowed = session.role === 'admin' || session.role === 'moderator'
  const handle = useHandleReport({
    mutation: {
      onSuccess: (result) => {
        setNotice(result.message)
        setTarget(null)
        void client.invalidateQueries({ queryKey: getListReportsQueryKey() })
      },
    },
  })

  function begin(report: ReportView, status: 'handled' | 'dismissed') {
    setTarget({ report, status })
    setNote('')
    setNotice('')
    setRequestId(crypto.randomUUID())
    handle.reset()
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (!target || handle.isPending || !allowed) return
    handle.mutate({
      data: { report_id: target.report.id, status: target.status, note: note.trim() },
      headers: { 'Idempotency-Key': requestId },
    })
  }

  return (
    <section aria-label="Player reports" className="connected-reports">
      <Panel
        title="Player reports"
        description="Players send these with /report in the game's chat. The reported player is never told."
      >
        {notice && (
          <p className="inline-notice notice-success" role="status">
            {notice}
          </p>
        )}
        <div className="table-toolbar">
          <Field id="report-filter" label="Show">
            <SelectControl
              id="report-filter"
              value={filter}
              onValueChange={(value) => setFilter(value as Filter)}
            >
              <SelectOption value="open">Open</SelectOption>
              <SelectOption value="handled">Handled</SelectOption>
              <SelectOption value="dismissed">Dismissed</SelectOption>
              <SelectOption value="all">All</SelectOption>
            </SelectControl>
          </Field>
          <span className="muted">
            {reports.data
              ? `${reports.data.open} open · checked ${new Date(reports.data.checked_at).toLocaleString()}`
              : 'Report list'}
          </span>
          <Button onClick={() => void reports.refetch()} disabled={reports.isFetching}>
            <RefreshCw size={15} />
            Refresh reports
          </Button>
        </div>
        {reports.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading reports…
          </p>
        )}
        {reports.error && (
          <p role="alert" className="connected-error">
            The reports could not be loaded. {errorText(reports.error)}
          </p>
        )}
        {reports.data && (
          <>
            <div className="table-scroll" role="region" aria-label="Reports table" tabIndex={0}>
              <table className="responsive-table">
                <thead>
                  <tr>
                    <th scope="col">Report</th>
                    <th scope="col">Sent</th>
                    <th scope="col">Reporter</th>
                    <th scope="col">About</th>
                    <th scope="col">Reason</th>
                    <th scope="col">Status</th>
                    <th scope="col">Handled by</th>
                    <th scope="col">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {reports.data.reports.map((r) => (
                    <tr key={r.id}>
                      <td className="row-heading numeric" data-label="Report">
                        #{r.id}
                      </td>
                      <td data-label="Sent">
                        <time dateTime={r.at}>{new Date(r.at).toLocaleString()}</time>
                      </td>
                      <td data-label="Reporter">
                        <Player name={r.reporter} user={r.reporter_user_id} />
                      </td>
                      <td data-label="About">
                        <Player name={r.target} user={r.target_user_id} />
                      </td>
                      <td data-label="Reason">{r.reason}</td>
                      <td data-label="Status">
                        <Badge tone={tone[r.status]}>{label[r.status]}</Badge>
                      </td>
                      <td data-label="Handled by">
                        {r.handled_by ? (
                          <>
                            {r.handled_by}
                            {r.note && <span className="block muted">{r.note}</span>}
                          </>
                        ) : (
                          'Nobody yet'
                        )}
                      </td>
                      <td data-label="Actions">
                        {r.status === 'open' ? (
                          <div className="action-group">
                            <Button
                              disabled={!allowed}
                              aria-label={`Handle report ${r.id}`}
                              onClick={() => begin(r, 'handled')}
                            >
                              Handle
                            </Button>
                            <Button
                              disabled={!allowed}
                              aria-label={`Dismiss report ${r.id}`}
                              onClick={() => begin(r, 'dismissed')}
                            >
                              Dismiss
                            </Button>
                          </div>
                        ) : (
                          'Closed'
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {reports.data.reports.length === 0 && (
              <p className="connected-state">
                {filter === 'open' ? 'No open reports.' : 'No reports in this list.'}
              </p>
            )}
          </>
        )}
      </Panel>
      <Modal
        open={target !== null}
        onOpenChange={(value) => {
          if (!value && !handle.isPending) setTarget(null)
        }}
        title={target?.status === 'dismissed' ? 'Dismiss report' : 'Handle report'}
        description={
          target
            ? `Report #${target.report.id}: ${target.report.reporter} about ${target.report.target}. Add what was done (optional).`
            : ''
        }
      >
        <form id="handle-report" onSubmit={submit} className="modal-body">
          <Field id="report-note" label="Note" hint="Up to 300 characters, shown with the report.">
            <Input
              id="report-note"
              maxLength={300}
              value={note}
              onChange={(e) => setNote(e.target.value)}
              disabled={handle.isPending}
            />
          </Field>
          {handle.error && (
            <div role="alert" className="connected-error">
              <p>{errorText(handle.error)}</p>
            </div>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">Admins and moderators</span>
          <div className="action-group">
            <Button disabled={handle.isPending} onClick={() => setTarget(null)}>
              Close
            </Button>
            <Button
              variant="primary"
              type="submit"
              form="handle-report"
              disabled={!allowed || handle.isPending}
            >
              {handle.isPending && <LoaderCircle size={16} className="animate-spin" />}
              {target?.status === 'dismissed' ? 'Dismiss report' : 'Mark handled'}
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  )
}
