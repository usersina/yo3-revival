import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, Plus, RefreshCw } from 'lucide-react'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import { Input, SelectControl, SelectOption } from './components/form-controls'
import {
  getListEventsQueryKey,
  useCancelEvent,
  useCreateEvent,
  useEditEvent,
  useListBanlists,
  useListEvents,
} from './api/generated/http/client'
import type { EventView, Kind, Phase, StaffSession } from './api/generated/http/models'
import { ApiHttpError } from './api/transport'

const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'

const kinds: { value: Kind; label: string; counts: string }[] = [
  { value: 'gunslinger', label: 'Gunslinger', counts: 'win streak' },
  { value: 'duelist-trial', label: 'Duelist Trial', counts: 'wins' },
  { value: 'evolution', label: 'Evolution Match', counts: 'stars' },
  { value: 'ultimate', label: 'Ultimate Match', counts: 'stars' },
  { value: 'anniversary', label: 'Anniversary Trial', counts: 'wins' },
]
const kindLabel = (kind: Kind) => kinds.find((k) => k.value === kind)?.label ?? kind
const tone = { scheduled: 'blue', running: 'green', finished: 'neutral', cancelled: 'red' } as const
const phaseLabel: Record<Phase, string> = {
  scheduled: 'Scheduled',
  running: 'Running',
  finished: 'Finished',
  cancelled: 'Cancelled',
}
const ANY = 'any'

/** An ISO time as a datetime-local input shows it (the browser's own zone). */
function local(iso: string) {
  const d = new Date(iso)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}
const soon = (hours: number) => local(new Date(Date.now() + hours * 3_600_000).toISOString())

type Form = {
  id: number | null
  kind: Kind
  name: string
  start: string
  end: string
  regulation: string
  participation: string
  threshold: string
  thresholdBp: string
}
const blank = (): Form => ({
  id: null,
  kind: 'duelist-trial',
  name: '',
  start: soon(0),
  end: soon(24 * 7),
  regulation: ANY,
  participation: '0',
  threshold: '0',
  thresholdBp: '0',
})
const fromEvent = (e: EventView): Form => ({
  id: e.event_id,
  kind: e.kind,
  name: e.name,
  start: local(e.start),
  end: local(e.end),
  regulation: e.regulation == null ? ANY : String(e.regulation),
  participation: String(e.prize.participation_bp ?? 0),
  threshold: String(e.prize.threshold ?? 0),
  thresholdBp: String(e.prize.threshold_bp ?? 0),
})

function prizeText(e: EventView) {
  const parts = []
  if (e.prize.participation_bp) parts.push(`${e.prize.participation_bp} BP for taking part`)
  if (e.prize.threshold && e.prize.threshold_bp)
    parts.push(`${e.prize.threshold_bp} BP at ${e.prize.threshold} ${e.counts}`)
  return parts.length ? parts.join(' · ') : 'No prize'
}

export function ConnectedEvents({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const events = useListEvents()
  const banlists = useListBanlists()
  const admin = session.role === 'admin'
  const [form, setForm] = useState<Form | null>(null)
  const [cancelling, setCancelling] = useState<EventView | null>(null)
  const [notice, setNotice] = useState('')
  const [requestId, setRequestId] = useState('')
  const refresh = () => client.invalidateQueries({ queryKey: getListEventsQueryKey() })
  const done = {
    onSuccess: (result: { message: string }) => {
      setNotice(result.message)
      setForm(null)
      setCancelling(null)
      void refresh()
    },
  }
  const create = useCreateEvent({ mutation: done })
  const edit = useEditEvent({ mutation: done })
  const cancel = useCancelEvent({ mutation: done })
  const saving = create.isPending || edit.isPending
  const problem = create.error ?? edit.error

  function begin(next: Form) {
    setForm(next)
    setNotice('')
    setRequestId(crypto.randomUUID())
    create.reset()
    edit.reset()
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (!form || saving || !admin) return
    const prize = {
      participation_bp: Number(form.participation) || 0,
      threshold: Number(form.threshold) || 0,
      threshold_bp: Number(form.thresholdBp) || 0,
    }
    const regulation = form.regulation === ANY ? null : Number(form.regulation)
    const times = {
      start: new Date(form.start).toISOString(),
      end: new Date(form.end).toISOString(),
    }
    if (form.id === null)
      create.mutate({
        data: { kind: form.kind, name: form.name.trim(), regulation, prize, ...times },
        headers: { 'Idempotency-Key': requestId },
      })
    else
      edit.mutate({
        data: {
          event_id: form.id,
          name: form.name.trim(),
          regulation: regulation ?? 0,
          prize,
          ...times,
        },
        headers: { 'Idempotency-Key': requestId },
      })
  }
  const set = (patch: Partial<Form>) => setForm((f) => (f ? { ...f, ...patch } : f))
  const counts = kinds.find((k) => k.value === form?.kind)?.counts ?? 'score'

  return (
    <section aria-label="Official events" className="connected-events">
      <Panel
        title="Official events"
        description="The Stadium on the map lists these. Players enter from its lobby and Participate pairs them with another entrant. Prizes are BP, sent by D-Mail when an event ends."
      >
        {notice && (
          <p className="inline-notice notice-success" role="status">
            {notice}
          </p>
        )}
        <div className="table-toolbar">
          <span className="muted">
            {events.data
              ? `${events.data.events.length} events · checked ${new Date(events.data.checked_at).toLocaleString()}`
              : 'Event list'}
          </span>
          <Button onClick={() => void events.refetch()} disabled={events.isFetching}>
            <RefreshCw size={15} />
            Refresh events
          </Button>
          <Button variant="primary" disabled={!admin} onClick={() => begin(blank())}>
            <Plus size={15} />
            Schedule event
          </Button>
        </div>
        {events.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading events…
          </p>
        )}
        {events.error && (
          <p role="alert" className="connected-error">
            The events could not be loaded. {errorText(events.error)}
          </p>
        )}
        {events.data && (
          <>
            <div className="table-scroll" role="region" aria-label="Events table" tabIndex={0}>
              <table className="responsive-table">
                <thead>
                  <tr>
                    <th scope="col">Event</th>
                    <th scope="col">Kind</th>
                    <th scope="col">When</th>
                    <th scope="col">Status</th>
                    <th scope="col">Entrants</th>
                    <th scope="col">Prize</th>
                    <th scope="col">Best scores</th>
                    <th scope="col">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {events.data.events.map((e) => (
                    <tr key={e.event_id}>
                      <td className="row-heading" data-label="Event">
                        {e.name}
                        <span className="block muted">#{e.event_id}</span>
                      </td>
                      <td data-label="Kind">
                        {kindLabel(e.kind)}
                        <span className="block muted">counts {e.counts}</span>
                      </td>
                      <td data-label="When">
                        <time dateTime={e.start}>{new Date(e.start).toLocaleString()}</time>
                        <span className="block muted">
                          to <time dateTime={e.end}>{new Date(e.end).toLocaleString()}</time>
                        </span>
                      </td>
                      <td data-label="Status">
                        <Badge tone={tone[e.phase]}>{phaseLabel[e.phase]}</Badge>
                      </td>
                      <td className="numeric" data-label="Entrants">
                        {e.entrants}
                      </td>
                      <td data-label="Prize">{prizeText(e)}</td>
                      <td data-label="Best scores">
                        {e.standings.length === 0
                          ? 'None yet'
                          : e.standings
                              .slice(0, 3)
                              .map((s) => `${s.name} ${s.best}`)
                              .join(' · ')}
                      </td>
                      <td data-label="Actions">
                        {e.phase === 'scheduled' || e.phase === 'running' ? (
                          <div className="action-group">
                            <Button
                              disabled={!admin}
                              aria-label={`Edit event ${e.event_id}`}
                              onClick={() => begin(fromEvent(e))}
                            >
                              Edit
                            </Button>
                            <Button
                              disabled={!admin}
                              aria-label={`Cancel event ${e.event_id}`}
                              onClick={() => {
                                setCancelling(e)
                                setNotice('')
                                setRequestId(crypto.randomUUID())
                                cancel.reset()
                              }}
                            >
                              Cancel
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
            {events.data.events.length === 0 && (
              <p className="connected-state">
                No official events yet. Schedule one and the Stadium lists it.
              </p>
            )}
          </>
        )}
      </Panel>

      <Modal
        open={form !== null}
        onOpenChange={(value) => {
          if (!value && !saving) setForm(null)
        }}
        title={form?.id === null ? 'Schedule event' : 'Edit event'}
        description="Times are in your browser's time zone. Prizes are BP, paid when the event ends."
      >
        <form id="event-form" onSubmit={submit} className="modal-body">
          <Field id="event-kind" label="Kind" hint={`The score counts ${counts}.`}>
            <SelectControl
              id="event-kind"
              value={form?.kind ?? 'duelist-trial'}
              onValueChange={(value) => set({ kind: value as Kind })}
              disabled={saving || form?.id !== null}
            >
              {kinds.map((k) => (
                <SelectOption key={k.value} value={k.value}>
                  {k.label}
                </SelectOption>
              ))}
            </SelectControl>
          </Field>
          <Field id="event-name" label="Name" hint="Shown to the staff and in prize D-Mails.">
            <Input
              id="event-name"
              required
              maxLength={60}
              value={form?.name ?? ''}
              onChange={(e) => set({ name: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="event-start" label="Starts">
            <Input
              id="event-start"
              type="datetime-local"
              required
              value={form?.start ?? ''}
              onChange={(e) => set({ start: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="event-end" label="Ends">
            <Input
              id="event-end"
              type="datetime-local"
              required
              value={form?.end ?? ''}
              onChange={(e) => set({ end: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field
            id="event-banlist"
            label="Banlist"
            hint="Entrants must log in with it (/banlist in the chat). Any lets everyone in."
          >
            <SelectControl
              id="event-banlist"
              value={form?.regulation ?? ANY}
              onValueChange={(value) => set({ regulation: value })}
              disabled={saving || !banlists.data}
            >
              <SelectOption value={ANY}>Any banlist</SelectOption>
              {banlists.data?.choices.map((c) => (
                <SelectOption key={c.id} value={String(c.id)}>
                  {c.name}
                </SelectOption>
              ))}
            </SelectControl>
          </Field>
          <Field
            id="event-participation"
            label="BP for taking part"
            hint="To everyone who plays a duel of it."
          >
            <Input
              id="event-participation"
              type="number"
              min={0}
              max={1000000}
              step={1}
              value={form?.participation ?? '0'}
              onChange={(e) => set({ participation: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field
            id="event-threshold"
            label={`Score that earns more (${counts})`}
            hint="0 for no such prize."
          >
            <Input
              id="event-threshold"
              type="number"
              min={0}
              max={9999}
              step={1}
              value={form?.threshold ?? '0'}
              onChange={(e) => set({ threshold: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="event-threshold-bp" label="BP for reaching it">
            <Input
              id="event-threshold-bp"
              type="number"
              min={0}
              max={1000000}
              step={1}
              value={form?.thresholdBp ?? '0'}
              onChange={(e) => set({ thresholdBp: e.target.value })}
              disabled={saving}
            />
          </Field>
          {problem && (
            <div role="alert" className="connected-error">
              <p>{errorText(problem)}</p>
            </div>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">Admins only</span>
          <div className="action-group">
            <Button disabled={saving} onClick={() => setForm(null)}>
              Close
            </Button>
            <Button variant="primary" type="submit" form="event-form" disabled={!admin || saving}>
              {saving && <LoaderCircle size={16} className="animate-spin" />}
              {form?.id === null ? 'Schedule event' : 'Save changes'}
            </Button>
          </div>
        </div>
      </Modal>

      <Modal
        open={cancelling !== null}
        onOpenChange={(value) => {
          if (!value && !cancel.isPending) setCancelling(null)
        }}
        title="Cancel event"
        description={
          cancelling
            ? `${kindLabel(cancelling.kind)} "${cancelling.name}" leaves the Stadium's list and nothing is paid. This cannot be undone.`
            : ''
        }
      >
        {cancel.error && (
          <div role="alert" className="connected-error">
            <p>{errorText(cancel.error)}</p>
          </div>
        )}
        <div className="modal-actions">
          <span className="muted">Admins only</span>
          <div className="action-group">
            <Button disabled={cancel.isPending} onClick={() => setCancelling(null)}>
              Keep event
            </Button>
            <Button
              variant="primary"
              disabled={!admin || cancel.isPending}
              onClick={() =>
                cancelling &&
                cancel.mutate({
                  data: { event_id: cancelling.event_id },
                  headers: { 'Idempotency-Key': requestId },
                })
              }
            >
              {cancel.isPending && <LoaderCircle size={16} className="animate-spin" />}
              Cancel event
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  )
}
