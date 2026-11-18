import { SelectControl, SelectOption, Input } from '../components/form-controls'
import { useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import {
  ArrowDown,
  ArrowUpRight,
  ChevronRight,
  Coins,
  ShieldCheck,
  Swords,
  Users,
} from 'lucide-react'
import { date, number } from '../data'
import type { Account } from '../data'
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
  SearchField,
} from '../components/ui'

export function AccountBadge({ account }: { account: Account }) {
  if (account.standing !== 'active')
    return (
      <Badge tone={account.standing === 'banned' ? 'red' : 'amber'} dot>
        {account.standing.charAt(0).toUpperCase() + account.standing.slice(1)}
      </Badge>
    )
  return (
    <Badge
      tone={
        account.presence === 'dueling'
          ? 'blue'
          : account.presence === 'online'
            ? 'green'
            : 'neutral'
      }
      dot
    >
      {account.presence === 'dueling'
        ? 'In a duel'
        : account.presence === 'online'
          ? 'Online'
          : 'Offline'}
    </Badge>
  )
}

function AccountDetails({ account, close }: { account: Account; close: () => void }) {
  const { change, notify, role } = usePreview()
  const [tab, setTab] = useState('overview')
  const [bp, setBp] = useState('500')
  const valid = /^\d+$/.test(bp) && +bp > 0 && +bp <= 1_000_000
  function gift(e: React.FormEvent) {
    e.preventDefault()
    if (!valid || role !== 'admin') return
    change(
      (s) => ({
        ...s,
        accounts: s.accounts.map((a) => (a.id === account.id ? { ...a, bp: a.bp + +bp } : a)),
      }),
      `Added ${number(+bp)} BP to ${account.user}`,
      account.user,
    )
    notify(`Added ${number(+bp)} BP to ${account.user}.`)
    setTab('overview')
  }
  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open) close()
      }}
      title={account.user}
      description={`User ID ${String(account.id).padStart(6, '0')} · Joined ${date(account.joined)}`}
      wide
    >
      <div className="account-summary">
        <Avatar initials={account.initials} color={account.color} />
        <div>
          <strong>{account.player}</strong>
          <span>{account.country}</span>
        </div>
        <AccountBadge account={account} />
      </div>
      <div className="modal-tabs">
        <button aria-pressed={tab === 'overview'} onClick={() => setTab('overview')}>
          Overview
        </button>
        <button
          aria-pressed={tab === 'bp'}
          disabled={role !== 'admin'}
          title={role !== 'admin' ? 'Administrators only' : undefined}
          onClick={() => setTab('bp')}
        >
          Give BP
        </button>
      </div>
      <div className="modal-body">
        {tab === 'overview' ? (
          <>
            <div className="detail-metrics">
              <div>
                <Coins size={18} />
                <strong>{number(account.bp)} BP</strong>
                <span>Current balance</span>
              </div>
              <div>
                <Swords size={18} />
                <strong>{account.wins + account.losses}</strong>
                <span>Duels played</span>
              </div>
              <div>
                <ShieldCheck size={18} />
                <strong>
                  {account.wins}W / {account.losses}L
                </strong>
                <span>Duel record</span>
              </div>
            </div>
            <dl className="detail-list">
              <div>
                <dt>Account role</dt>
                <dd>
                  {account.role === 'admin'
                    ? 'Administrator'
                    : account.role === 'moderator'
                      ? 'Moderator'
                      : 'Player'}
                </dd>
              </div>
              <div>
                <dt>Standing</dt>
                <dd>{account.standing === 'active' ? 'Good standing' : account.standing}</dd>
              </div>
              <div>
                <dt>Player name</dt>
                <dd>{account.player}</dd>
              </div>
            </dl>
          </>
        ) : (
          <form id="gift-bp" onSubmit={gift}>
            <Field
              id="bp-amount"
              label="BP amount"
              hint="Added to the existing balance. The change is recorded in the audit log."
            >
              <Input
                id="bp-amount"
                type="number"
                min="1"
                max="1000000"
                required
                value={bp}
                onChange={(e) => setBp(e.target.value)}
              />
            </Field>
            <p className="balance-preview">
              New balance <strong>{number(account.bp + (valid ? +bp : 0))} BP</strong>
            </p>
          </form>
        )}
      </div>
      <div className="modal-actions">
        {account.role === 'player' ? (
          <Link to={`/moderation?account=${account.id}`} className="text-link" onClick={close}>
            <ShieldCheck size={15} />
            Open moderation
            <ArrowUpRight size={14} />
          </Link>
        ) : (
          <span className="modal-context muted">Staff account</span>
        )}
        <div className="action-group">
          <Button onClick={close}>Close</Button>
          {tab === 'bp' && (
            <Button
              variant="primary"
              type="submit"
              form="gift-bp"
              disabled={!valid || role !== 'admin'}
            >
              <Coins size={16} />
              Add BP
            </Button>
          )}
        </div>
      </div>
    </Modal>
  )
}

export function Accounts() {
  const { data } = usePreview()
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState('all')
  const [sort, setSort] = useState('joined')
  const [searchParams, setSearchParams] = useSearchParams()
  const selected = data.accounts.find((a) => a.id === Number(searchParams.get('account')))
  const accounts = data.accounts
    .filter(
      (a) =>
        `${a.user} ${a.player} ${a.id}`.toLowerCase().includes(query.toLowerCase().trim()) &&
        (status === 'all' ||
          (status === 'online'
            ? a.presence !== 'offline'
            : status === 'offline'
              ? a.presence === 'offline'
              : a.standing !== 'active')),
    )
    .sort((a, b) =>
      sort === 'bp'
        ? b.bp - a.bp
        : sort === 'name'
          ? a.user.localeCompare(b.user)
          : b.joined.localeCompare(a.joined),
    )
  const filtered = query !== '' || status !== 'all'
  return (
    <>
      <PageHeading
        title="Accounts"
        description="The people behind the duels. Find an account and take a closer look."
      />
      <div className="section-summary">
        <span>
          <Users size={17} />
          <strong>{data.accounts.length}</strong> accounts
        </span>
        <span>
          <i className="legend-dot" />
          <strong>{data.accounts.filter((a) => a.presence !== 'offline').length}</strong> online now
        </span>
        <span>
          <ShieldCheck size={16} />
          <strong>{data.accounts.filter((a) => a.role !== 'player').length}</strong> staff members
        </span>
      </div>
      <Panel>
        <div className="table-toolbar">
          <SearchField
            label="Search accounts"
            placeholder="Search by User ID or player name…"
            value={query}
            onChange={setQuery}
          />
          <div className="filter-controls">
            <label className="sr-only" htmlFor="account-status">
              Account status
            </label>
            <SelectControl
              id="account-status"
              value={status}
              onValueChange={(value) => setStatus(value)}
            >
              <SelectOption value="all">All statuses</SelectOption>
              <SelectOption value="online">Online</SelectOption>
              <SelectOption value="offline">Offline</SelectOption>
              <SelectOption value="flagged">Needs attention</SelectOption>
            </SelectControl>
            <label className="sr-only" htmlFor="account-sort">
              Sort accounts
            </label>
            <SelectControl id="account-sort" value={sort} onValueChange={(value) => setSort(value)}>
              <SelectOption value="joined">Newest first</SelectOption>
              <SelectOption value="name">Name A–Z</SelectOption>
              <SelectOption value="bp">Highest BP</SelectOption>
            </SelectControl>
          </div>
        </div>
        {accounts.length ? (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Account</th>
                    <th scope="col">Player name</th>
                    <th scope="col">Status</th>
                    <th scope="col" className="numeric">
                      Balance
                    </th>
                    <th scope="col">
                      Joined <ArrowDown size={12} className="inline-icon" />
                    </th>
                    <th scope="col">
                      <span className="sr-only">Details</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {accounts.map((a) => (
                    <tr key={a.id}>
                      <td>
                        <button
                          className="account-cell"
                          aria-label={`View ${a.user}`}
                          onClick={() => setSearchParams({ account: String(a.id) })}
                        >
                          <Avatar initials={a.initials} color={a.color} />
                          <span>
                            <strong>
                              {a.user}
                              {a.role !== 'player' && (
                                <ShieldCheck size={12} className="staff-icon" />
                              )}
                            </strong>
                            <small>#{String(a.id).padStart(6, '0')}</small>
                          </span>
                        </button>
                      </td>
                      <td>{a.player}</td>
                      <td>
                        <AccountBadge account={a} />
                      </td>
                      <td className="numeric tabular-nums">
                        {number(a.bp)} <span className="muted">BP</span>
                      </td>
                      <td className="muted nowrap">{date(a.joined)}</td>
                      <td>
                        <Button
                          variant="ghost"
                          className="icon-button"
                          aria-label={`Account details for ${a.user}`}
                          onClick={() => setSearchParams({ account: String(a.id) })}
                        >
                          <ChevronRight size={15} />
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="table-footer">
              <span>
                Showing {accounts.length} of {data.accounts.length} accounts
              </span>
              {filtered && (
                <button
                  className="text-button"
                  onClick={() => {
                    setQuery('')
                    setStatus('all')
                  }}
                >
                  Clear filters
                </button>
              )}
            </div>
          </>
        ) : (
          <EmptyState
            title="No accounts found"
            description="Try another name or include accounts with a different status."
            action={
              <Button
                onClick={() => {
                  setQuery('')
                  setStatus('all')
                }}
              >
                Clear filters
              </Button>
            }
          />
        )}
      </Panel>
      {selected && (
        <AccountDetails
          key={selected.id}
          account={selected}
          close={() => setSearchParams({}, { replace: true })}
        />
      )}
    </>
  )
}
