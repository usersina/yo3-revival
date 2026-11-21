import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, Plus, RefreshCw } from 'lucide-react'
import { Badge, Button, Field, Modal, Panel } from './components/ui'
import { Input } from './components/form-controls'
import {
  getListChampionshipsQueryKey,
  useCancelChampionship,
  useCreateChampionship,
  useEditChampionship,
  useListChampionships,
} from './api/generated/http/client'
import type { ChampionshipState, ChampionshipView, StaffSession } from './api/generated/http/models'
import { ApiHttpError } from './api/transport'

const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'

const tone = { scheduled: 'blue', running: 'green', finished: 'neutral', cancelled: 'red' } as const
const stateLabel: Record<ChampionshipState, string> = {
  scheduled: 'Scheduled',
  running: 'Running',
  finished: 'Finished',
  cancelled: 'Cancelled',
}

/** An ISO time as a datetime-local input shows it (the browser's own zone). */
function local(iso: string) {
  const d = new Date(iso)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}
const soon = (hours: number) => local(new Date(Date.now() + hours * 3_600_000).toISOString())

type Form = {
  id: number | null
  name: string
  open: string
  close: string
  start: string
  minDuels: string
  participation: string
  finalist: string
  winner: string
}
const blank = (): Form => ({
  id: null,
  name: '',
  open: soon(0),
  close: soon(4),
  start: soon(5),
  minDuels: '0',
  participation: '0',
  finalist: '0',
  winner: '0',
})
const fromChampionship = (c: ChampionshipView): Form => ({
  id: c.championship_id,
  name: c.name,
  open: local(c.entry_open),
  close: local(c.entry_close),
  start: local(c.start),
  minDuels: String(c.min_duels),
  participation: String(c.prize.participation_bp ?? 0),
  finalist: String(c.prize.finalist_bp ?? 0),
  winner: String(c.prize.winner_bp ?? 0),
})

function prizeText(c: ChampionshipView) {
  const parts = []
  if (c.prize.participation_bp) parts.push(`${c.prize.participation_bp} BP for being drawn`)
  if (c.prize.finalist_bp) parts.push(`${c.prize.finalist_bp} BP to each finalist`)
  if (c.prize.winner_bp) parts.push(`${c.prize.winner_bp} BP to the winner`)
  return parts.length ? parts.join(' · ') : 'No prize'
}

export function ConnectedChampionships({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const list = useListChampionships()
  const admin = session.role === 'admin'
  const [form, setForm] = useState<Form | null>(null)
  const [cancelling, setCancelling] = useState<ChampionshipView | null>(null)
  const [notice, setNotice] = useState('')
  const [requestId, setRequestId] = useState('')
  const refresh = () => client.invalidateQueries({ queryKey: getListChampionshipsQueryKey() })
  const done = {
    onSuccess: (result: { message: string }) => {
      setNotice(result.message)
      setForm(null)
      setCancelling(null)
      void refresh()
    },
  }
  const create = useCreateChampionship({ mutation: done })
  const edit = useEditChampionship({ mutation: done })
  const cancel = useCancelChampionship({ mutation: done })
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
      finalist_bp: Number(form.finalist) || 0,
      winner_bp: Number(form.winner) || 0,
    }
    const times = {
      entry_open: new Date(form.open).toISOString(),
      entry_close: new Date(form.close).toISOString(),
      start: new Date(form.start).toISOString(),
    }
    const min_duels = Number(form.minDuels) || 0
    if (form.id === null)
      create.mutate({
        data: { name: form.name.trim(), min_duels, prize, ...times },
        headers: { 'Idempotency-Key': requestId },
      })
    else
      edit.mutate({
        data: { championship_id: form.id, name: form.name.trim(), min_duels, prize, ...times },
        headers: { 'Idempotency-Key': requestId },
      })
  }
  const set = (patch: Partial<Form>) => setForm((f) => (f ? { ...f, ...patch } : f))

  return (
    <section aria-label="Championship" className="connected-events">
      <Panel
        title="Championship"
        description="The map's Tournament tile. Players enter by opening its lobby while entries are open; at the start time the server draws up to 16 of them and plays a single-day knockout Final, with byes filling the empty seats. Prizes are BP, sent by D-Mail."
      >
        {notice && (
          <p className="inline-notice notice-success" role="status">
            {notice}
          </p>
        )}
        <div className="table-toolbar">
          <span className="muted">
            {list.data
              ? `${list.data.championships.length} Championships · checked ${new Date(list.data.checked_at).toLocaleString()}`
              : 'Championship list'}
          </span>
          <Button onClick={() => void list.refetch()} disabled={list.isFetching}>
            <RefreshCw size={15} />
            Refresh Championships
          </Button>
          <Button variant="primary" disabled={!admin} onClick={() => begin(blank())}>
            <Plus size={15} />
            Schedule Championship
          </Button>
        </div>
        {list.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading Championships…
          </p>
        )}
        {list.error && (
          <p role="alert" className="connected-error">
            The Championships could not be loaded. {errorText(list.error)}
          </p>
        )}
        {list.data && (
          <>
            <div
              className="table-scroll"
              role="region"
              aria-label="Championships table"
              tabIndex={0}
            >
              <table className="responsive-table">
                <thead>
                  <tr>
                    <th scope="col">Championship</th>
                    <th scope="col">Entries</th>
                    <th scope="col">Starts</th>
                    <th scope="col">Status</th>
                    <th scope="col">Entrants</th>
                    <th scope="col">Prize</th>
                    <th scope="col">Result</th>
                    <th scope="col">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {list.data.championships.length === 0 && (
                    <tr>
                      <td className="connected-state" colSpan={8}>
                        No Championship yet. Schedule one and the map's Tournament tile lists it.
                      </td>
                    </tr>
                  )}
                  {list.data.championships.map((c) => (
                    <tr key={c.championship_id}>
                      <td className="row-heading" data-label="Championship">
                        {c.name}
                        <span className="block muted">
                          Y.O.C. {c.year} season {c.season} · #{c.championship_id}
                        </span>
                      </td>
                      <td data-label="Entries">
                        <time dateTime={c.entry_open}>
                          {new Date(c.entry_open).toLocaleString()}
                        </time>
                        <span className="block muted">
                          to{' '}
                          <time dateTime={c.entry_close}>
                            {new Date(c.entry_close).toLocaleString()}
                          </time>
                          {c.min_duels > 0 && ` · ${c.min_duels}+ duels`}
                        </span>
                      </td>
                      <td data-label="Starts">
                        <time dateTime={c.start}>{new Date(c.start).toLocaleString()}</time>
                      </td>
                      <td data-label="Status">
                        <Badge tone={tone[c.state]}>{stateLabel[c.state]}</Badge>
                      </td>
                      <td className="numeric" data-label="Entrants">
                        {c.entrants.length}
                      </td>
                      <td data-label="Prize">{prizeText(c)}</td>
                      <td data-label="Result">
                        {c.winner
                          ? `${c.winner.name} won${c.second ? `, ${c.second.name} second` : ''}`
                          : c.state === 'finished'
                            ? 'No winner'
                            : 'Not played yet'}
                      </td>
                      <td data-label="Actions">
                        {c.state === 'scheduled' ? (
                          <div className="action-group">
                            <Button
                              disabled={!admin}
                              aria-label={`Edit Championship ${c.championship_id}`}
                              onClick={() => begin(fromChampionship(c))}
                            >
                              Edit
                            </Button>
                            <Button
                              disabled={!admin}
                              aria-label={`Cancel Championship ${c.championship_id}`}
                              onClick={() => {
                                setCancelling(c)
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
          </>
        )}
      </Panel>

      <Modal
        open={form !== null}
        onOpenChange={(value) => {
          if (!value && !saving) setForm(null)
        }}
        title={form?.id === null ? 'Schedule Championship' : 'Edit Championship'}
        description="Times are in your browser's time zone. Entries open, close and the draw all fall within one day. Fewer than two entrants cancels it."
      >
        <form id="championship-form" onSubmit={submit} className="modal-body">
          <Field id="championship-name" label="Name" hint="Shown to the staff and in D-Mails.">
            <Input
              id="championship-name"
              required
              maxLength={60}
              value={form?.name ?? ''}
              onChange={(e) => set({ name: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-open" label="Entries open">
            <Input
              id="championship-open"
              type="datetime-local"
              required
              value={form?.open ?? ''}
              onChange={(e) => set({ open: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-close" label="Entries close">
            <Input
              id="championship-close"
              type="datetime-local"
              required
              value={form?.close ?? ''}
              onChange={(e) => set({ close: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-start" label="Draw and first round">
            <Input
              id="championship-start"
              type="datetime-local"
              required
              value={form?.start ?? ''}
              onChange={(e) => set({ start: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field
            id="championship-min-duels"
            label="Minimum duels played"
            hint="A player must have played this many duels to enter. 0 lets everyone in."
          >
            <Input
              id="championship-min-duels"
              type="number"
              min={0}
              step={1}
              value={form?.minDuels ?? '0'}
              onChange={(e) => set({ minDuels: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-participation" label="BP for being drawn">
            <Input
              id="championship-participation"
              type="number"
              min={0}
              max={1000000}
              step={1}
              value={form?.participation ?? '0'}
              onChange={(e) => set({ participation: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-finalist" label="BP to each finalist">
            <Input
              id="championship-finalist"
              type="number"
              min={0}
              max={1000000}
              step={1}
              value={form?.finalist ?? '0'}
              onChange={(e) => set({ finalist: e.target.value })}
              disabled={saving}
            />
          </Field>
          <Field id="championship-winner" label="BP to the winner">
            <Input
              id="championship-winner"
              type="number"
              min={0}
              max={1000000}
              step={1}
              value={form?.winner ?? '0'}
              onChange={(e) => set({ winner: e.target.value })}
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
            <Button
              variant="primary"
              type="submit"
              form="championship-form"
              disabled={!admin || saving}
            >
              {saving && <LoaderCircle size={16} className="animate-spin" />}
              {form?.id === null ? 'Schedule Championship' : 'Save changes'}
            </Button>
          </div>
        </div>
      </Modal>

      <Modal
        open={cancelling !== null}
        onOpenChange={(value) => {
          if (!value && !cancel.isPending) setCancelling(null)
        }}
        title="Cancel Championship"
        description={
          cancelling
            ? `"${cancelling.name}" leaves the map's list and its ${cancelling.entrants.length} entrants are not drawn. This cannot be undone.`
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
              Keep Championship
            </Button>
            <Button
              variant="primary"
              disabled={!admin || cancel.isPending}
              onClick={() =>
                cancelling &&
                cancel.mutate({
                  data: { championship_id: cancelling.championship_id },
                  headers: { 'Idempotency-Key': requestId },
                })
              }
            >
              {cancel.isPending && <LoaderCircle size={16} className="animate-spin" />}
              Cancel Championship
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  )
}
