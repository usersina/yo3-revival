import { SelectControl, SelectOption, Input, Textarea } from '../components/form-controls'
import { useState } from 'react'
import {
  CalendarClock,
  Check,
  Eye,
  FileText,
  Megaphone,
  Pencil,
  Plus,
  Save,
  Settings2,
  Trash2,
  Wrench,
} from 'lucide-react'
import { id, timestamp } from '../data'
import type { News } from '../data'
import { usePreview } from '../preview'
import {
  Button,
  EmptyState,
  Field,
  Modal,
  PageHeading,
  Panel,
  ReadOnlyNote,
  SavedMark,
} from '../components/ui'

export function SettingsPage() {
  const { data, draft, setDraft, role, change, notify } = usePreview()
  const dirty = JSON.stringify(draft) !== JSON.stringify(data.settings)
  const admin = role === 'admin'
  const errors = {
    name: draft.name.trim() ? '' : 'Enter a server name.',
    lp:
      Number.isInteger(draft.lp) && draft.lp >= 100 && draft.lp <= 999_999
        ? ''
        : 'Enter a whole number from 100 to 999,999.',
    turn:
      Number.isInteger(draft.turn) && draft.turn >= 0 && draft.turn <= 3600
        ? ''
        : 'Enter a whole number from 0 to 3,600 seconds.',
    total:
      Number.isInteger(draft.total) && draft.total >= 0 && draft.total <= 86_400
        ? ''
        : 'Enter a whole number from 0 to 86,400 seconds.',
  }
  const valid = !Object.values(errors).some(Boolean)
  function save(e: React.FormEvent) {
    e.preventDefault()
    if (!valid || !admin || !dirty) return
    const next = { ...draft, name: draft.name.trim() }
    change((s) => ({ ...s, settings: next }), 'Updated server settings', next.name)
    setDraft(next)
    notify('Settings saved. Changes apply in the preview.')
  }
  return (
    <>
      <PageHeading
        title="Settings"
        description="Make this server yours. Keep the defaults that already work."
      />
      {!admin && <ReadOnlyNote>Only administrators can change server settings.</ReadOnlyNote>}
      <form onSubmit={save}>
        <fieldset disabled={!admin} className="settings-fieldset">
          <Panel
            title="Server identity"
            description="The name and welcome message your players see."
            action={
              <span className="section-icon">
                <Settings2 size={19} />
              </span>
            }
          >
            <div className="settings-section">
              <div className="setting-row">
                <div>
                  <label htmlFor="server-name">Server name</label>
                  <p>A short, recognisable name for your community.</p>
                </div>
                <div>
                  <Input
                    id="server-name"
                    required
                    maxLength={64}
                    value={draft.name}
                    aria-invalid={Boolean(errors.name)}
                    aria-describedby={errors.name ? 'server-name-error' : undefined}
                    onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                  />
                  {errors.name && (
                    <p className="field-error" id="server-name-error" role="alert">
                      {errors.name}
                    </p>
                  )}
                </div>
              </div>
              <div className="setting-row">
                <div>
                  <label htmlFor="server-headline">Welcome headline</label>
                  <p>Shown to players when they join.</p>
                </div>
                <Input
                  id="server-headline"
                  maxLength={128}
                  value={draft.headline}
                  onChange={(e) => setDraft({ ...draft, headline: e.target.value })}
                />
              </div>
            </div>
          </Panel>
          <Panel title="Joining the server" description="Choose who can create an account.">
            <div className="settings-section">
              <div className="setting-row">
                <div>
                  <label htmlFor="registration">Registration</label>
                  <p>Existing players can still sign in when registration is closed.</p>
                </div>
                <SelectControl
                  id="registration"
                  value={draft.registration}
                  onValueChange={(value) =>
                    setDraft({
                      ...draft,
                      registration: value as typeof draft.registration,
                    })
                  }
                >
                  <SelectOption value="invite">Invite only</SelectOption>
                  <SelectOption value="open">Open to everyone</SelectOption>
                  <SelectOption value="closed">Closed</SelectOption>
                </SelectControl>
              </div>
            </div>
          </Panel>
          <Panel
            title="Duel defaults"
            description="Starting rules for new duels. Duels already in progress keep their rules."
          >
            <div className="settings-section">
              <div className="setting-row">
                <div>
                  <label htmlFor="starting-lp">Starting Life Points</label>
                  <p>The familiar 8,000 LP is a good place to start.</p>
                </div>
                <div>
                  <div className="input-unit">
                    <Input
                      id="starting-lp"
                      type="number"
                      min="100"
                      max="999999"
                      required
                      value={Number.isNaN(draft.lp) ? '' : draft.lp}
                      aria-invalid={Boolean(errors.lp)}
                      aria-describedby={errors.lp ? 'starting-lp-error' : undefined}
                      onChange={(e) => setDraft({ ...draft, lp: e.target.valueAsNumber })}
                    />
                    <span>LP</span>
                  </div>
                  {errors.lp && (
                    <p className="field-error" id="starting-lp-error" role="alert">
                      {errors.lp}
                    </p>
                  )}
                </div>
              </div>
              <div className="setting-row">
                <div>
                  <label htmlFor="turn-time">Turn timer</label>
                  <p>Seconds per turn. Set to 0 to turn it off.</p>
                </div>
                <div>
                  <div className="input-unit">
                    <Input
                      id="turn-time"
                      type="number"
                      min="0"
                      max="3600"
                      required
                      value={Number.isNaN(draft.turn) ? '' : draft.turn}
                      aria-invalid={Boolean(errors.turn)}
                      aria-describedby={errors.turn ? 'turn-time-error' : undefined}
                      onChange={(e) => setDraft({ ...draft, turn: e.target.valueAsNumber })}
                    />
                    <span>seconds</span>
                  </div>
                  {errors.turn && (
                    <p className="field-error" id="turn-time-error" role="alert">
                      {errors.turn}
                    </p>
                  )}
                </div>
              </div>
              <div className="setting-row">
                <div>
                  <label htmlFor="total-time">Total timer</label>
                  <p>Seconds per player for the whole duel. 0 means no limit.</p>
                </div>
                <div>
                  <div className="input-unit">
                    <Input
                      id="total-time"
                      type="number"
                      min="0"
                      max="86400"
                      required
                      value={Number.isNaN(draft.total) ? '' : draft.total}
                      aria-invalid={Boolean(errors.total)}
                      aria-describedby={errors.total ? 'total-time-error' : undefined}
                      onChange={(e) => setDraft({ ...draft, total: e.target.valueAsNumber })}
                    />
                    <span>seconds</span>
                  </div>
                  {errors.total && (
                    <p className="field-error" id="total-time-error" role="alert">
                      {errors.total}
                    </p>
                  )}
                </div>
              </div>
              <div className="setting-row">
                <div>
                  <label htmlFor="regulation">Default regulation</label>
                  <p>Card limits applied when players sign in.</p>
                </div>
                <Input
                  id="regulation"
                  value={draft.regulation}
                  readOnly
                  aria-describedby="regulation-note"
                />
              </div>
              <p id="regulation-note" className="field-hint">
                The available regulations will come from the server’s game data.
              </p>
            </div>
          </Panel>
        </fieldset>
        <div className="settings-savebar">
          <span>
            {dirty ? (
              <>
                <span className="unsaved-dot" />
                Unsaved changes
              </>
            ) : (
              <SavedMark />
            )}
          </span>
          <div className="action-group">
            <Button disabled={!dirty || !admin} onClick={() => setDraft(data.settings)}>
              Discard changes
            </Button>
            <Button variant="primary" type="submit" disabled={!dirty || !valid || !admin}>
              <Save size={16} />
              Save changes
            </Button>
          </div>
        </div>
      </form>
      <p className="settings-note">
        Ports, certificates and file paths belong in the server configuration. This page is for live
        settings.
      </p>
    </>
  )
}

function NewsEditor({ entry, close }: { entry: News | null; close: () => void }) {
  const { change, notify, data } = usePreview()
  const [title, setTitle] = useState(entry?.title ?? '')
  const [body, setBody] = useState(entry?.body ?? '')
  const [preview, setPreview] = useState(false)
  const valid =
    title.trim().length > 0 && body.trim().length > 0 && (entry !== null || data.news.length < 10)
  function save(e: React.FormEvent) {
    e.preventDefault()
    if (!valid) return
    const next = {
      id: entry?.id ?? id(),
      title: title.trim(),
      body: body.trim(),
      at: new Date().toISOString(),
    }
    change(
      (s) => ({
        ...s,
        news: entry ? s.news.map((n) => (n.id === entry.id ? next : n)) : [next, ...s.news],
      }),
      entry ? 'Updated a news entry' : 'Published a news entry',
      next.title,
    )
    notify(entry ? 'News entry updated.' : 'News entry published in the preview.')
    close()
  }
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v) close()
      }}
      title={entry ? 'Edit news entry' : 'New announcement'}
      description="Appears in the game’s Information list. Keep it short and useful."
      wide
    >
      <form onSubmit={save}>
        <div className="modal-tabs">
          <button type="button" aria-pressed={!preview} onClick={() => setPreview(false)}>
            <Pencil size={14} />
            Write
          </button>
          <button type="button" aria-pressed={preview} onClick={() => setPreview(true)}>
            <Eye size={14} />
            Preview
          </button>
        </div>
        <div className="modal-body form-stack">
          {preview ? (
            <div className="news-preview">
              <span>INFORMATION</span>
              <h3>{title || 'Your announcement title'}</h3>
              <p>{body || 'Your message will appear here.'}</p>
            </div>
          ) : (
            <>
              <Field id="news-title" label="Title">
                <Input
                  id="news-title"
                  maxLength={128}
                  required
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="What’s happening?"
                />
              </Field>
              <Field
                id="news-body"
                label="Message"
                hint={`${body.length}/2,048 characters. Plain text, just like the game.`}
              >
                <Textarea
                  id="news-body"
                  rows={6}
                  maxLength={2048}
                  required
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  placeholder="Something your players should know…"
                />
              </Field>
            </>
          )}
        </div>
        <div className="modal-actions">
          <Button onClick={close}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={!valid}>
            <Megaphone size={16} />
            {entry ? 'Save entry' : 'Publish announcement'}
          </Button>
        </div>
      </form>
    </Modal>
  )
}

export function NewsPage() {
  const { data, role, change, notify } = usePreview()
  const [editor, setEditor] = useState<News | 'new' | null>(null)
  const [remove, setRemove] = useState<News | null>(null)
  const [maintenance, setMaintenance] = useState(false)
  const [minutes, setMinutes] = useState('15')
  const [message, setMessage] = useState('A quick server update. We’ll be back shortly.')
  const [confirmed, setConfirmed] = useState(false)
  const admin = role === 'admin'
  const valid = /^\d+$/.test(minutes) && +minutes <= 1440 && message.trim().length > 0
  function schedule(e: React.FormEvent) {
    e.preventDefault()
    if (!valid || !admin) return
    if (!confirmed) {
      setConfirmed(true)
      return
    }
    const nextMaintenance = {
      at: new Date(Date.now() + +minutes * 60_000).toISOString(),
      message: message.trim(),
    }
    change(
      (s) => ({
        ...s,
        maintenance: nextMaintenance,
      }),
      'Scheduled maintenance',
      `${minutes} minutes`,
    )
    notify('Maintenance scheduled in the preview.')
    setMaintenance(false)
    setConfirmed(false)
  }
  return (
    <>
      <PageHeading
        title="News & maintenance"
        description="Keep your players in the loop, in the game itself."
        action={
          admin ? (
            <Button
              variant="primary"
              disabled={data.news.length >= 10}
              onClick={() => setEditor('new')}
            >
              <Plus size={16} />
              New announcement
            </Button>
          ) : undefined
        }
      />
      {!admin && (
        <ReadOnlyNote>Only administrators can publish news or schedule maintenance.</ReadOnlyNote>
      )}
      <Panel
        title="In-game announcements"
        description={`${data.news.length} of 10 entries · shown in the Information list`}
      >
        <div className="news-list">
          {data.news.map((n) => (
            <article className="news-entry" key={n.id}>
              <span className="news-icon">
                <FileText size={19} />
              </span>
              <div>
                <h3>{n.title}</h3>
                <p>{n.body}</p>
                <time dateTime={n.at}>{timestamp(n.at)}</time>
              </div>
              {admin && (
                <div className="action-group">
                  <Button
                    variant="ghost"
                    className="icon-button"
                    onClick={() => setEditor(n)}
                    aria-label={`Edit ${n.title}`}
                  >
                    <Pencil size={16} />
                  </Button>
                  <Button
                    variant="ghost"
                    className="icon-button danger-text"
                    onClick={() => setRemove(n)}
                    aria-label={`Delete ${n.title}`}
                  >
                    <Trash2 size={16} />
                  </Button>
                </div>
              )}
            </article>
          ))}
        </div>
        {!data.news.length && (
          <EmptyState
            title="Nothing announced yet"
            description="A short welcome message is a good place to start."
          />
        )}
      </Panel>
      <Panel
        title="Maintenance"
        description="Give players a heads-up before you take a break."
        action={<Wrench size={18} className="muted" />}
      >
        <div className="maintenance-content">
          <span className={`round-icon ${data.maintenance ? 'amber' : 'green'}`}>
            <CalendarClock size={22} />
          </span>
          <div>
            <h3>
              {data.maintenance
                ? `Scheduled for ${timestamp(data.maintenance.at)}`
                : 'No maintenance scheduled'}
            </h3>
            <p>{data.maintenance?.message ?? 'Players can sign in and duel as usual.'}</p>
          </div>
          {admin &&
            (data.maintenance ? (
              <Button
                onClick={() => {
                  change((s) => ({ ...s, maintenance: null }), 'Cancelled maintenance')
                  notify('Maintenance cancelled.')
                }}
              >
                Cancel maintenance
              </Button>
            ) : (
              <Button onClick={() => setMaintenance(true)}>Schedule maintenance</Button>
            ))}
        </div>
      </Panel>
      {editor !== null && (
        <NewsEditor entry={editor === 'new' ? null : editor} close={() => setEditor(null)} />
      )}
      <Modal
        open={remove !== null}
        onOpenChange={(v) => {
          if (!v) setRemove(null)
        }}
        title="Delete this announcement?"
        description={`“${remove?.title ?? ''}” will be removed from the in-game Information list.`}
      >
        <div className="modal-actions">
          <Button onClick={() => setRemove(null)}>Cancel</Button>
          <Button
            variant="danger"
            onClick={() => {
              if (remove) {
                change(
                  (s) => ({ ...s, news: s.news.filter((n) => n.id !== remove.id) }),
                  'Deleted a news entry',
                  remove.title,
                )
                notify('Announcement deleted.')
                setRemove(null)
              }
            }}
          >
            <Trash2 size={16} />
            Delete announcement
          </Button>
        </div>
      </Modal>
      <Modal
        open={maintenance}
        onOpenChange={(v) => {
          setMaintenance(v)
          if (!v) setConfirmed(false)
        }}
        title="Schedule maintenance"
        description="The game counts down, then only staff can sign in. This does not stop the server process."
      >
        <form onSubmit={schedule}>
          <div className="modal-body form-stack">
            {confirmed ? (
              <div className="confirmation-summary">
                <Wrench size={22} />
                <div>
                  <h3>
                    {+minutes === 0
                      ? 'Start maintenance now?'
                      : `Start maintenance in ${minutes} minutes?`}
                  </h3>
                  <p>{message}</p>
                </div>
              </div>
            ) : (
              <>
                <Field
                  id="maintenance-minutes"
                  label="Start in"
                  hint="Minutes from now. 0 starts immediately."
                >
                  <div className="input-unit">
                    <Input
                      id="maintenance-minutes"
                      type="number"
                      min="0"
                      max="1440"
                      required
                      value={minutes}
                      onChange={(e) => setMinutes(e.target.value)}
                    />
                    <span>minutes</span>
                  </div>
                </Field>
                <Field id="maintenance-message" label="Player message">
                  <Textarea
                    id="maintenance-message"
                    rows={3}
                    maxLength={200}
                    required
                    value={message}
                    onChange={(e) => setMessage(e.target.value)}
                  />
                </Field>
              </>
            )}
          </div>
          <div className="modal-actions">
            <Button onClick={confirmed ? () => setConfirmed(false) : () => setMaintenance(false)}>
              {confirmed ? 'Back' : 'Cancel'}
            </Button>
            <Button variant={confirmed ? 'danger' : 'primary'} type="submit" disabled={!valid}>
              {confirmed ? (
                <>
                  <Check size={16} />
                  Confirm maintenance
                </>
              ) : (
                'Review schedule'
              )}
            </Button>
          </div>
        </form>
      </Modal>
    </>
  )
}
