import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { CalendarClock, LoaderCircle, Plus, RefreshCw, Trash2 } from 'lucide-react'
import { z } from 'zod'
import { Badge, Button, Field, Modal, Panel, ReadOnlyNote, SearchField } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  getListSettingsQueryKey,
  useChangeSettings,
  useListSettings,
} from './api/generated/http/client'
import type { ChangeSettingsMutationVariables } from './api/generated/http/client'
import type {
  SettingRow,
  SettingsRequest,
  SettingsResult,
  StaffSession,
} from './api/generated/http/models'
import {
  ChangeSettingsHeader,
  SettingsRequest as RequestSchema,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

const pendingSchema = z.object({ data: RequestSchema, headers: ChangeSettingsHeader })
const newsSchema = z
  .array(z.object({ title: z.string(), body: z.string() }))
  .min(1)
  .max(10)
const newsText = z
  .string()
  .transform((value, ctx) => {
    try {
      return JSON.parse(value) as unknown
    } catch {
      ctx.addIssue({ code: 'custom', message: 'Invalid announcements' })
      return z.NEVER
    }
  })
  .pipe(newsSchema)
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Retry this same change to confirm its outcome.'

export function ConnectedSettings({
  session,
  view = 'all',
}: {
  session: StaffSession
  view?: 'all' | 'news' | 'registry'
}) {
  const client = useQueryClient()
  const list = useListSettings()
  const storageKey = `yo3:pending-settings:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error: 'Could not restore a settings change. Check the audit log before making another.',
      }
    }
  })
  const [pending, setPending] = useState<ChangeSettingsMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState<SettingsRequest | null>(null)
  const [row, setRow] = useState<SettingRow | null>(null)
  const [accepted, setAccepted] = useState(false)
  const [query, setQuery] = useState('')
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const [result, setResult] = useState<SettingsResult | null>(null)
  const mutation = useChangeSettings({
    mutation: {
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        try {
          sessionStorage.removeItem(storageKey)
          setPending(null)
        } catch {
          setStorageError(
            'The change was saved, but this browser could not clear its retry request.',
          )
        }
        void client.invalidateQueries({ queryKey: getListSettingsQueryKey() })
      },
      onError: (error) => {
        setFailure(errorText(error))
        setRefused(
          error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status),
        )
        if (error instanceof ApiHttpError && [401, 403].includes(error.status))
          void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
      },
    },
  })
  const admin = session.role === 'admin'
  const busy = mutation.isPending
  const blocked =
    !admin || Boolean(pending || storageError) || busy || list.isFetching || Boolean(list.error)
  const current = list.data
  const request = pending?.data ?? draft
  const setting =
    row ??
    current?.settings.find(
      (s) => (request?.kind === 'set' || request?.kind === 'reset') && s.name === request.name,
    )
  const news =
    request?.kind === 'set' && setting?.input.kind === 'news'
      ? newsText.safeParse(request.value)
      : null
  function begin(data: SettingsRequest, selected: SettingRow | null = null) {
    setDraft(data)
    setRow(selected)
    setAccepted(false)
    setFailure('')
    setResult(null)
    setRefused(false)
    mutation.reset()
    setOpen(true)
  }
  function edit() {
    if (!pending || !refused) return
    try {
      sessionStorage.removeItem(storageKey)
      setDraft(pending.data)
      setPending(null)
      setFailure('')
      setRefused(false)
      setAccepted(false)
      mutation.reset()
    } catch {
      setStorageError('Could not clear the refused request. No new change was sent.')
    }
  }
  function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || result || storageError || !admin || (!pending && !accepted)) return
    let variables = pending
    if (!variables) {
      if (!draft) return
      variables = { data: draft, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      if (!pendingSchema.safeParse(variables).success) return
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this change safely. No request was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  function updateValue(value: string) {
    if (draft?.kind === 'set') {
      setDraft({ ...draft, value })
      setAccepted(false)
    }
  }
  const visible = current?.settings.filter(
    (s) =>
      (view !== 'news' || s.input.kind === 'news') &&
      (view !== 'registry' || s.input.kind !== 'news') &&
      `${s.name} ${s.help}`.toLowerCase().includes(query.trim().toLowerCase()),
  )
  return (
    <Panel
      className="connected-panel"
      title={view === 'news' ? 'News & maintenance' : 'Server settings'}
      description="Live settings are saved without restarting the server. Each setting says when it applies."
      action={
        <div className="action-group">
          {pending && <Button onClick={() => setOpen(true)}>Resume settings change</Button>}
          <Button
            aria-label="Refresh settings"
            onClick={() => void list.refetch()}
            disabled={list.isFetching || busy}
          >
            <RefreshCw size={16} className={list.isFetching ? 'spin' : undefined} />
          </Button>
        </div>
      }
    >
      {!admin && <ReadOnlyNote>Only administrators can change server settings.</ReadOnlyNote>}
      {storageError && (
        <p role="alert" className="connected-error">
          {storageError}
        </p>
      )}
      {list.isPending && (
        <p className="connected-state">
          <LoaderCircle size={16} className="spin" /> Loading settings…
        </p>
      )}
      {list.error && (
        <p role="alert" className="connected-error">
          {errorText(list.error)} Use Refresh settings to try again.
        </p>
      )}
      {current && (
        <>
          {view !== 'registry' && (
            <div className="settings-maintenance-row">
              <div>
                <h3>
                  <CalendarClock size={18} /> Maintenance{' '}
                  <Badge
                    tone={
                      current.maintenance_active
                        ? 'red'
                        : current.maintenance_at
                          ? 'amber'
                          : 'green'
                    }
                  >
                    {current.maintenance_active
                      ? 'Active'
                      : current.maintenance_at
                        ? 'Scheduled'
                        : 'Off'}
                  </Badge>
                </h3>
                <p>
                  {current.maintenance_at
                    ? `Starts ${new Date(current.maintenance_at).toLocaleString()}. Only staff can stay signed in during maintenance.`
                    : 'Players can sign in and play.'}
                </p>
              </div>
              <div className="action-group">
                <Button
                  disabled={blocked}
                  onClick={() =>
                    begin({
                      kind: 'schedule_maintenance',
                      minutes: 30,
                      message:
                        current.settings.find((s) => s.name === 'maintenance-message')?.value ?? '',
                    })
                  }
                >
                  Schedule maintenance
                </Button>
                {current.maintenance_at && (
                  <Button disabled={blocked} onClick={() => begin({ kind: 'cancel_maintenance' })}>
                    Cancel maintenance
                  </Button>
                )}
              </div>
            </div>
          )}
          <p className="field-hint">
            Checked {new Date(current.checked_at).toLocaleString()}. Refresh to check the current
            maintenance status.
          </p>
          <details className="connected-settings-list" open={view !== 'all'}>
            <summary>{view === 'news' ? 'Game announcements' : 'All live settings'}</summary>
            <SearchField
              value={query}
              onChange={setQuery}
              placeholder="Find a setting…"
              label="Find a setting"
            />
            <div className="settings-section">
              {visible?.map((s) => (
                <div key={s.name} className="setting-row" data-testid={`setting-${s.name}`}>
                  <div>
                    <strong>{s.name}</strong>
                    <p>{s.help}</p>
                    <p>
                      Applies {s.applies} · {s.overridden ? 'Custom value' : 'Default value'}
                    </p>
                  </div>
                  <div className="setting-value">
                    <code>
                      {s.input.kind === 'news' ? 'Game announcements' : s.value || '(none)'}
                    </code>
                    <div className="action-group">
                      <Button
                        disabled={blocked}
                        onClick={() => begin({ kind: 'set', name: s.name, value: s.value }, s)}
                      >
                        Edit
                      </Button>
                      <Button
                        disabled={blocked || !s.overridden}
                        onClick={() => begin({ kind: 'reset', name: s.name }, s)}
                      >
                        Reset
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
              {visible?.length === 0 && (
                <p className="connected-state">No matching settings. Try another name.</p>
              )}
            </div>
          </details>
        </>
      )}
      <Modal
        open={open}
        onOpenChange={(value) => {
          if (!busy) setOpen(value)
        }}
        title={
          request?.kind === 'cancel_maintenance'
            ? 'Cancel maintenance'
            : request?.kind === 'schedule_maintenance'
              ? 'Schedule maintenance'
              : `${request?.kind === 'reset' ? 'Reset' : 'Edit'} ${setting?.name ?? 'setting'}`
        }
        description={
          pending
            ? 'A saved request is waiting for confirmation. Retry it with the same request ID.'
            : 'Changes are saved with an audit entry. Review the effect before confirming.'
        }
      >
        <form id="connected-settings-change" onSubmit={submit} className="modal-body">
          {result ? (
            <p role="status" className="inline-notice">
              {result.message}
            </p>
          ) : (
            <>
              {request && (
                <fieldset className="settings-fieldset" disabled={busy || Boolean(pending)}>
                  {request.kind === 'set' && setting && (
                    <>
                      <p>
                        {setting.help} Applies {setting.applies}.
                      </p>
                      {setting.input.kind === 'news' ? (
                        <>
                          <p className="field-hint">
                            1–10 announcements. The game needs at least one.
                          </p>
                          {news && !news.success && (
                            <p role="alert" className="connected-error">
                              Could not read these announcements. Close this dialog and refresh
                              settings.
                            </p>
                          )}
                          {news?.success &&
                            news.data.map((entry, index) => (
                              <div className="settings-news-entry" key={index}>
                                <Field id={`live-news-title-${index}`} label={`Title ${index + 1}`}>
                                  <input
                                    id={`live-news-title-${index}`}
                                    value={entry.title}
                                    maxLength={128}
                                    required
                                    onChange={(e) =>
                                      updateValue(
                                        JSON.stringify(
                                          news.data.map((v, i) =>
                                            i === index ? { ...v, title: e.target.value } : v,
                                          ),
                                        ),
                                      )
                                    }
                                  />
                                </Field>
                                <Field id={`live-news-body-${index}`} label={`Body ${index + 1}`}>
                                  <textarea
                                    id={`live-news-body-${index}`}
                                    value={entry.body}
                                    maxLength={2048}
                                    rows={3}
                                    onChange={(e) =>
                                      updateValue(
                                        JSON.stringify(
                                          news.data.map((v, i) =>
                                            i === index ? { ...v, body: e.target.value } : v,
                                          ),
                                        ),
                                      )
                                    }
                                  />
                                </Field>
                                <Button
                                  disabled={news.data.length === 1}
                                  onClick={() =>
                                    updateValue(
                                      JSON.stringify(news.data.filter((_, i) => i !== index)),
                                    )
                                  }
                                >
                                  <Trash2 size={14} /> Remove announcement {index + 1}
                                </Button>
                              </div>
                            ))}
                          {news?.success && (
                            <Button
                              disabled={news.data.length >= 10}
                              onClick={() =>
                                updateValue(JSON.stringify([...news.data, { title: '', body: '' }]))
                              }
                            >
                              <Plus size={14} /> Add announcement
                            </Button>
                          )}
                        </>
                      ) : (
                        <Field id="live-setting-value" label="Value">
                          {setting.input.kind === 'choice' ? (
                            <select
                              id="live-setting-value"
                              value={request.value}
                              onChange={(e) => updateValue(e.target.value)}
                            >
                              {setting.input.choices.map((c) => (
                                <option key={c} value={c}>
                                  {c}
                                </option>
                              ))}
                            </select>
                          ) : setting.input.kind === 'regulation' &&
                            setting.input.choices.length > 0 ? (
                            <select
                              id="live-setting-value"
                              value={request.value}
                              onChange={(e) => updateValue(e.target.value)}
                            >
                              <option value="">Default regulation</option>
                              {!setting.input.choices.some((c) => String(c.id) === request.value) &&
                                request.value && (
                                  <option value={request.value}>{request.value}</option>
                                )}
                              {setting.input.choices.map((c) => (
                                <option key={c.id} value={c.id}>
                                  {c.name}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <input
                              id="live-setting-value"
                              type={setting.input.kind === 'number' ? 'number' : 'text'}
                              value={request.value}
                              min={setting.input.kind === 'number' ? setting.input.min : undefined}
                              max={setting.input.kind === 'number' ? setting.input.max : undefined}
                              step={setting.input.kind === 'number' ? 1 : undefined}
                              maxLength={
                                setting.input.kind === 'text' ? setting.input.max_length : undefined
                              }
                              required={setting.input.kind === 'number'}
                              onChange={(e) => updateValue(e.target.value)}
                            />
                          )}
                        </Field>
                      )}
                    </>
                  )}
                  {request.kind === 'reset' && (
                    <p>
                      Restore {setting?.name ?? request.name} to{' '}
                      {setting?.default_value || '(none)'}. Applies{' '}
                      {setting?.applies ?? 'when the setting takes effect'}.
                    </p>
                  )}
                  {request.kind === 'schedule_maintenance' && (
                    <>
                      <p className="inline-notice">
                        When maintenance starts, player duels end and everyone except staff is
                        signed out. Set 0 to start now.
                      </p>
                      <Field id="maintenance-minutes" label="Starts in (minutes)">
                        <input
                          id="maintenance-minutes"
                          type="number"
                          min={0}
                          max={
                            current
                              ? Math.floor(
                                  (4294967295 - Date.parse(current.checked_at) / 1000) / 60,
                                )
                              : undefined
                          }
                          step={1}
                          value={Number.isNaN(request.minutes) ? '' : request.minutes}
                          required
                          onChange={(e) => {
                            setDraft({ ...request, minutes: e.target.valueAsNumber })
                            setAccepted(false)
                          }}
                        />
                      </Field>
                      <Field id="maintenance-message" label="Message to players">
                        <input
                          id="maintenance-message"
                          value={request.message ?? ''}
                          maxLength={200}
                          onChange={(e) => {
                            setDraft({ ...request, message: e.target.value })
                            setAccepted(false)
                          }}
                        />
                      </Field>
                    </>
                  )}
                  {request.kind === 'cancel_maintenance' && (
                    <p>
                      Allow normal sign-ins again. Players already seeing the game’s maintenance
                      icon keep it until their next login.
                    </p>
                  )}
                  <label className="profile-role-confirm">
                    <input
                      type="checkbox"
                      checked={accepted}
                      onChange={(e) => setAccepted(e.target.checked)}
                    />{' '}
                    Confirm this change
                  </label>
                </fieldset>
              )}
            </>
          )}
          {failure && (
            <div role="alert" className="connected-error">
              <p>{failure}</p>
              {pending && !refused && <p>Retry the same change to confirm its outcome.</p>}
            </div>
          )}
          {pending && (
            <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions">
          <Button disabled={busy} onClick={() => setOpen(false)}>
            {result ? 'Done' : 'Close'}
          </Button>
          {pending && refused && <Button onClick={edit}>Edit request</Button>}
          {!result && (
            <Button
              variant="primary"
              type="submit"
              form="connected-settings-change"
              disabled={
                busy ||
                Boolean(storageError) ||
                !admin ||
                Boolean(pending && refused) ||
                (!pending && !accepted)
              }
            >
              {busy ? (
                <>
                  <LoaderCircle size={16} className="spin" /> Saving…
                </>
              ) : pending ? (
                'Retry same change'
              ) : (
                'Save change'
              )}
            </Button>
          )}
        </div>
      </Modal>
    </Panel>
  )
}
