import { SelectControl, SelectOption, Textarea } from '../components/form-controls'
import { useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import {
  AlertTriangle,
  ArrowDownToLine,
  Check,
  Clock3,
  DatabaseBackup,
  FileText,
  Filter,
  Plus,
  ShieldCheck,
  ShieldOff,
  Terminal,
  TriangleAlert,
} from 'lucide-react'
import { date, id, time, timestamp } from '../data'
import { downloadText } from '../download'
import { usePreview } from '../preview'
import {
  Avatar,
  Badge,
  Button,
  EmptyState,
  Field,
  Modal,
  PageHeading,
  Panel,
  ReadOnlyNote,
  SearchField,
} from '../components/ui'
import { AccountBadge } from './Accounts'

const actions: Record<string, { label: string; verb: string; result: string; help: string }> = {
  mute: {
    label: 'Mute chat',
    verb: 'Mute',
    result: 'muted',
    help: 'The player can keep dueling but cannot send chat messages.',
  },
  unmute: {
    label: 'Unmute chat',
    verb: 'Unmute',
    result: 'unmuted',
    help: 'Allow the player to send chat messages again.',
  },
  kick: {
    label: 'Disconnect',
    verb: 'Disconnect',
    result: 'disconnected',
    help: 'Disconnect the player now. They can sign in again.',
  },
  restrict: {
    label: 'Restrict account',
    verb: 'Restrict',
    result: 'restricted',
    help: 'Prevent this account from signing in until the restriction is lifted.',
  },
  lift: {
    label: 'Lift restrictions',
    verb: 'Lift restrictions on',
    result: 'restored to good standing',
    help: 'Remove the account’s ban or restriction.',
  },
  ban: {
    label: 'Ban account',
    verb: 'Ban',
    result: 'banned',
    help: 'Prevent this account from signing in. This can be lifted later.',
  },
  'sign-out': {
    label: 'Sign out all devices',
    verb: 'Sign out',
    result: 'signed out on all devices',
    help: 'Revoke saved device sessions and disconnect the account.',
  },
}

function ModerationDialog({
  open,
  close,
  initialAccount,
}: {
  open: boolean
  close: () => void
  initialAccount?: string
}) {
  const { data, change, notify } = usePreview()
  const [accountId, setAccountId] = useState(initialAccount ?? '1')
  const [action, setAction] = useState('mute')
  const [reason, setReason] = useState('')
  const [reviewing, setReviewing] = useState(false)
  const account = data.accounts.find((a) => String(a.id) === accountId)
  const valid =
    account?.role === 'player' && reason.trim().length >= 3 && reason.trim().length <= 200
  function apply() {
    if (!valid || !account) return
    change(
      (s) => ({
        ...s,
        accounts: s.accounts.map((a) =>
          a.id !== account.id
            ? a
            : {
                ...a,
                standing:
                  action === 'ban'
                    ? 'banned'
                    : action === 'restrict'
                      ? 'restricted'
                      : action === 'mute'
                        ? 'muted'
                        : action === 'unmute' || action === 'lift'
                          ? 'active'
                          : a.standing,
                presence: ['ban', 'restrict', 'kick', 'sign-out'].includes(action)
                  ? 'offline'
                  : a.presence,
              },
        ),
      }),
      `${actions[action].verb} ${account.user} — ${reason.trim()}`,
      account.user,
    )
    notify(`${account.user} was ${actions[action].result}.`)
    close()
  }
  return (
    <Modal
      open={open}
      onOpenChange={(v) => {
        if (!v) close()
      }}
      title="Moderation action"
      description="Choose an account and review the effect before confirming."
    >
      <form
        onSubmit={(e) => {
          e.preventDefault()
          if (valid) setReviewing(true)
        }}
      >
        <div className="modal-body form-stack">
          {reviewing ? (
            <>
              <div
                className={`confirmation-summary ${action === 'ban' ? 'confirmation-danger' : ''}`}
              >
                <AlertTriangle size={22} />
                <div>
                  <h3>
                    {actions[action].verb} {account?.user}?
                  </h3>
                  <p>{actions[action].help}</p>
                </div>
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Reason</dt>
                  <dd>{reason}</dd>
                </div>
                <div>
                  <dt>Recorded as</dt>
                  <dd>Staff action in the audit log</dd>
                </div>
              </dl>
            </>
          ) : (
            <>
              <Field id="moderation-account" label="Account">
                <SelectControl
                  id="moderation-account"
                  value={accountId}
                  onValueChange={(value) => setAccountId(value)}
                >
                  {data.accounts
                    .filter((a) => a.role === 'player')
                    .map((a) => (
                      <SelectOption value={a.id} key={a.id}>
                        {a.user}
                      </SelectOption>
                    ))}
                </SelectControl>
              </Field>
              <Field id="moderation-action" label="Action">
                <SelectControl
                  id="moderation-action"
                  value={action}
                  onValueChange={(value) => setAction(value)}
                >
                  {Object.entries(actions).map(([key, value]) => (
                    <SelectOption key={key} value={key}>
                      {value.label}
                    </SelectOption>
                  ))}
                </SelectControl>
              </Field>
              <p className="field-hint">{actions[action].help}</p>
              <Field
                id="moderation-reason"
                label="Reason"
                hint="3–200 characters. Keep it useful for the next moderator."
              >
                <Textarea
                  id="moderation-reason"
                  rows={3}
                  minLength={3}
                  maxLength={200}
                  required
                  placeholder="What happened?"
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                />
              </Field>
            </>
          )}
        </div>
        <div className="modal-actions">
          <Button onClick={reviewing ? () => setReviewing(false) : close}>
            {reviewing ? 'Back' : 'Cancel'}
          </Button>
          {reviewing ? (
            <Button
              variant={['ban', 'restrict'].includes(action) ? 'danger' : 'primary'}
              disabled={!valid}
              onClick={apply}
            >
              Confirm {action === 'sign-out' ? 'sign-out' : action}
            </Button>
          ) : (
            <Button variant="primary" type="submit" disabled={!valid}>
              Review action
            </Button>
          )}
        </div>
      </form>
    </Modal>
  )
}

export function ModerationPage() {
  const { data } = usePreview()
  const [params] = useSearchParams()
  const requested = params.get('account') ?? undefined
  const [open, setOpen] = useState(!!requested)
  const flagged = data.accounts.filter((a) => a.standing !== 'active')
  return (
    <>
      <PageHeading
        title="Moderation"
        description="Keep the community a good place to play. Every action is recorded."
        action={
          <Button variant="primary" onClick={() => setOpen(true)}>
            <ShieldCheck size={17} />
            Take action
          </Button>
        }
      />
      <div className="moderation-summary">
        <div>
          <span className="round-icon amber">
            <ShieldOff size={20} />
          </span>
          <div>
            <strong>{flagged.length}</strong>
            <span>Accounts with active restrictions</span>
          </div>
        </div>
        <Link to="/audit" className="text-link">
          See the full audit log
          <ArrowDownToLine size={15} />
        </Link>
      </div>
      <Panel
        title="Active restrictions"
        description="Review an account before changing its standing."
      >
        {flagged.length ? (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Account</th>
                  <th scope="col">Player</th>
                  <th scope="col">Standing</th>
                  <th scope="col">Account details</th>
                </tr>
              </thead>
              <tbody>
                {flagged.map((a) => (
                  <tr key={a.id}>
                    <td>
                      <span className="account-cell">
                        <Avatar initials={a.initials} color={a.color} />
                        <strong>{a.user}</strong>
                      </span>
                    </td>
                    <td>{a.player}</td>
                    <td>
                      <AccountBadge account={a} />
                    </td>
                    <td>
                      <Link className="text-link" to={`/accounts?account=${a.id}`}>
                        View account
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState
            title="All in good standing"
            description="No accounts are currently banned, restricted or muted."
          />
        )}
      </Panel>
      <div className="context-note">
        <ShieldCheck size={19} />
        <div>
          <strong>Actions should be easy to explain.</strong>
          <p>
            Give a clear reason, check the account, and confirm. Staff account changes will stay
            behind administrator permissions.
          </p>
        </div>
      </div>
      {open && <ModerationDialog open close={() => setOpen(false)} initialAccount={requested} />}
    </>
  )
}

export function AuditPage() {
  const { data } = usePreview()
  const [query, setQuery] = useState('')
  const [source, setSource] = useState('all')
  const rows = data.audit.filter(
    (a) =>
      `${a.actor} ${a.action} ${a.target}`.toLowerCase().includes(query.trim().toLowerCase()) &&
      (source === 'all' || a.source === source),
  )
  return (
    <>
      <PageHeading
        title="Audit log"
        description="Who changed what, and when. Across the dashboard, CLI and server."
        action={
          <Button
            onClick={() =>
              downloadText(
                'yo3-preview-audit.json',
                JSON.stringify({ preview: true, entries: rows }, null, 2),
                'application/json',
              )
            }
          >
            <ArrowDownToLine size={16} />
            Export log
          </Button>
        }
      />
      <Panel>
        <div className="table-toolbar">
          <SearchField
            label="Search audit log"
            placeholder="Search staff, actions or accounts…"
            value={query}
            onChange={setQuery}
          />
          <div className="filter-controls">
            <Filter size={15} className="muted" />
            <label htmlFor="audit-source" className="sr-only">
              Audit source
            </label>
            <SelectControl
              id="audit-source"
              value={source}
              onValueChange={(value) => setSource(value)}
            >
              <SelectOption value="all">All sources</SelectOption>
              <SelectOption value="Dashboard">Dashboard</SelectOption>
              <SelectOption value="CLI">CLI</SelectOption>
              <SelectOption value="Server">Server</SelectOption>
            </SelectControl>
          </div>
        </div>
        {rows.length ? (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Time</th>
                  <th scope="col">Staff member</th>
                  <th scope="col">Action</th>
                  <th scope="col">Source</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((a) => (
                  <tr key={a.id}>
                    <td className="nowrap">
                      <strong className="table-emphasis tabular-nums">{time(a.at)}</strong>
                      <small className="cell-subtitle">{date(a.at)}</small>
                    </td>
                    <td>
                      <span className="actor-cell">
                        <span
                          className={`actor-dot ${a.actor === 'System' ? 'actor-system' : ''}`}
                        />
                        {a.actor}
                      </span>
                    </td>
                    <td>
                      <span className="audit-action">{a.action}</span>
                      <small className="cell-subtitle">{a.target}</small>
                    </td>
                    <td>
                      <Badge>{a.source}</Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState
            description="Try another search or include a different source."
            action={
              <Button
                onClick={() => {
                  setQuery('')
                  setSource('all')
                }}
              >
                Clear filters
              </Button>
            }
          />
        )}
        <div className="table-footer">
          <span>{rows.length} entries</span>
          <span>
            <ShieldCheck size={14} />
            Staff changes are recorded automatically
          </span>
        </div>
      </Panel>
    </>
  )
}

export function LogsPage() {
  const { data } = usePreview()
  const [query, setQuery] = useState('')
  const [level, setLevel] = useState('all')
  const rows = data.logs.filter(
    (l) =>
      `${l.module} ${l.message}`.toLowerCase().includes(query.trim().toLowerCase()) &&
      (level === 'all' || l.level === level),
  )
  return (
    <>
      <PageHeading
        title="Server logs"
        description="What the server is doing, in its own words."
        action={
          <Button
            onClick={() =>
              downloadText(
                'yo3-preview-server.log',
                [
                  '# YO3 dashboard preview — sample logs only',
                  ...rows.map((l) => `${l.at} ${l.level.toUpperCase()} ${l.module}: ${l.message}`),
                ].join('\n'),
              )
            }
          >
            <ArrowDownToLine size={16} />
            Export logs
          </Button>
        }
      />
      <Panel>
        <div className="table-toolbar">
          <SearchField
            label="Search server logs"
            placeholder="Search messages or modules…"
            value={query}
            onChange={setQuery}
          />
          <div className="filter-controls">
            <label htmlFor="log-level" className="sr-only">
              Log level
            </label>
            <SelectControl id="log-level" value={level} onValueChange={(value) => setLevel(value)}>
              <SelectOption value="all">All levels</SelectOption>
              <SelectOption value="info">Information</SelectOption>
              <SelectOption value="warning">Warnings</SelectOption>
              <SelectOption value="error">Errors</SelectOption>
            </SelectControl>
            <Badge tone="neutral">
              <Terminal size={12} />
              Sample log
            </Badge>
          </div>
        </div>
        {rows.length ? (
          <div className="log-list">
            {rows.map((l) => (
              <div className={`log-line log-${l.level}`} key={l.id}>
                <time dateTime={l.at}>{time(l.at)}</time>
                <Badge
                  tone={l.level === 'warning' ? 'amber' : l.level === 'error' ? 'red' : 'neutral'}
                >
                  {l.level === 'warning' ? 'WARN' : l.level.toUpperCase()}
                </Badge>
                <code>{l.module}</code>
                <p>{l.message}</p>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            title={
              level === 'error' && !query
                ? 'No errors in the sample log'
                : 'No matching log entries'
            }
            description="Try another level or clear your search."
            action={
              <Button
                onClick={() => {
                  setQuery('')
                  setLevel('all')
                }}
              >
                Clear filters
              </Button>
            }
          />
        )}
        <div className="table-footer">
          <span>{rows.length} entries · newest first</span>
          <span>Times shown in your local timezone</span>
        </div>
      </Panel>
      <div className="context-note">
        <TriangleAlert size={18} />
        <div>
          <strong>Sharing a log?</strong>
          <p>
            Check it for player names, addresses and other private information before attaching it
            to a public issue.
          </p>
        </div>
      </div>
    </>
  )
}

export function BackupsPage() {
  const { data, role, change, notify } = usePreview()
  function create() {
    const snapshot = {
      id: id(),
      at: new Date().toISOString(),
      size: 'Preview',
      kind: 'Manual' as const,
      preview: true,
    }
    change(
      (s) => ({
        ...s,
        backups: [snapshot, ...s.backups],
      }),
      'Created a manual backup',
      'server.db',
    )
    notify('Sample backup added. No database file was created.', 'info')
  }
  return (
    <>
      <PageHeading
        title="Backups"
        description="A way back, when you need one. Keep your players’ progress safe."
        action={
          role === 'admin' ? (
            <Button variant="primary" onClick={create}>
              <Plus size={16} />
              Create backup
            </Button>
          ) : undefined
        }
      />
      <div className="backup-summary">
        <span className="round-icon green">
          <DatabaseBackup size={23} />
        </span>
        <div>
          <h2>Automatic backups are enabled</h2>
          <p>At startup, every hour and on shutdown. The newest 24 are kept.</p>
        </div>
        <Badge tone="green">
          <Check size={13} />
          Saving normally
        </Badge>
      </div>
      {role !== 'admin' && (
        <ReadOnlyNote>Only administrators can create or download backups.</ReadOnlyNote>
      )}
      <Panel
        title="Database snapshots"
        description="Stored on the server. Download a copy to keep it somewhere else."
      >
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">Snapshot</th>
                <th scope="col">Created</th>
                <th scope="col">Size</th>
                <th scope="col">Type</th>
                <th scope="col">
                  <span className="sr-only">Download</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {data.backups.map((b, index) => (
                <tr key={b.id}>
                  <td>
                    <div className="snapshot-cell">
                      <FileText size={19} />
                      <div>
                        <strong>server.db</strong>
                        <small>
                          {index === 0
                            ? 'Latest snapshot'
                            : `Snapshot ${data.backups.length - index}`}
                        </small>
                      </div>
                    </div>
                  </td>
                  <td className="nowrap">{timestamp(b.at)}</td>
                  <td className="tabular-nums">{b.size}</td>
                  <td>
                    <Badge tone={b.kind === 'Manual' ? 'blue' : 'neutral'}>{b.kind}</Badge>
                  </td>
                  <td>
                    {role === 'admin' && (
                      <Button
                        variant="ghost"
                        className="download-preview"
                        disabled
                        title="Available once the dashboard is connected to the server"
                        aria-label={`Download snapshot from ${timestamp(b.at)} (requires server connection)`}
                      >
                        <ArrowDownToLine size={15} />
                        Download
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="table-footer">
          <span>{data.backups.length} sample snapshots</span>
          <span>
            <Clock3 size={14} />
            Downloads need a server connection
          </span>
        </div>
      </Panel>
      <div className="context-note">
        <DatabaseBackup size={19} />
        <div>
          <strong>A backup on the same machine is a starting point.</strong>
          <p>
            Keep an off-server copy as well. Restoring a real database requires stopping the server
            first.
          </p>
          <a
            className="text-link"
            href="https://github.com/usersina/yo3-revival/blob/beta/docs/hosting.md"
            target="_blank"
            rel="noreferrer"
          >
            Read the hosting guide
          </a>
        </div>
      </div>
    </>
  )
}
