import { useState } from 'react'
import { CalendarClock, Check, Coins, Plus, Ticket, Trash2 } from 'lucide-react'
import { usePreview } from '../preview'
import { date, number } from '../data'
import type { Reward } from '../data'
import {
  Badge,
  Button,
  CopyButton,
  EmptyState,
  Field,
  Modal,
  PageHeading,
  Panel,
  ReadOnlyNote,
} from '../components/ui'
import { InviteDialog } from '../components/InviteDialog'

function RewardDialog({ open, close }: { open: boolean; close: () => void }) {
  const { change, notify, data } = usePreview()
  const [code, setCode] = useState('')
  const [bp, setBp] = useState('500')
  const [limit, setLimit] = useState('20')
  const duplicate = data.rewards.some((r) => r.code.toLowerCase() === code.trim().toLowerCase())
  const valid =
    /^[A-Za-z0-9-]{3,32}$/.test(code.trim()) &&
    /^\d+$/.test(bp) &&
    +bp > 0 &&
    +bp <= 1_000_000 &&
    /^\d+$/.test(limit) &&
    +limit > 0 &&
    +limit <= 100_000 &&
    !duplicate
  function submit(e: React.FormEvent) {
    e.preventDefault()
    if (!valid) return
    const value = code.trim().toUpperCase()
    change(
      (s) => ({
        ...s,
        rewards: [{ code: value, bp: +bp, claims: 0, limit: +limit }, ...s.rewards],
      }),
      'Created a BOOSTERPASS code',
      value,
    )
    notify('BOOSTERPASS code created in the preview.')
    close()
  }
  return (
    <Modal
      open={open}
      onOpenChange={(v) => {
        if (!v) close()
      }}
      title="Create BP code"
      description="Players redeem this code in Main Menu → BOOSTERPASS, once per account."
    >
      <form onSubmit={submit}>
        <div className="modal-body form-stack">
          <Field id="reward-code" label="Code" hint="3–32 letters, numbers or hyphens.">
            <input
              id="reward-code"
              autoComplete="off"
              maxLength={32}
              value={code}
              placeholder="WEEKEND-DUELS"
              onChange={(e) => setCode(e.target.value)}
              aria-invalid={duplicate}
            />
          </Field>
          {duplicate && (
            <p className="field-error" role="alert">
              This code already exists. Choose another.
            </p>
          )}
          <div className="form-grid">
            <Field id="reward-bp" label="BP per account">
              <input
                id="reward-bp"
                type="number"
                min="1"
                max="1000000"
                value={bp}
                onChange={(e) => setBp(e.target.value)}
              />
            </Field>
            <Field id="reward-limit" label="Maximum claims">
              <input
                id="reward-limit"
                type="number"
                min="1"
                max="100000"
                value={limit}
                onChange={(e) => setLimit(e.target.value)}
              />
            </Field>
          </div>
        </div>
        <div className="modal-actions">
          <Button onClick={close}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={!valid}>
            <Coins size={16} />
            Create code
          </Button>
        </div>
      </form>
    </Modal>
  )
}

export function Invites() {
  const { data, role, change, notify } = usePreview()
  const [tab, setTab] = useState('invites')
  const [invite, setInvite] = useState(false)
  const [reward, setReward] = useState(false)
  const [remove, setRemove] = useState<Reward | null>(null)
  const [now] = useState(Date.now)
  return (
    <>
      <PageHeading
        title="Invites & codes"
        description="Welcome new duelists, or give the regulars a little something."
        action={
          tab === 'invites' ? (
            <Button variant="primary" onClick={() => setInvite(true)}>
              <Plus size={17} />
              Create invite
            </Button>
          ) : role === 'admin' ? (
            <Button variant="primary" onClick={() => setReward(true)}>
              <Plus size={17} />
              Create BP code
            </Button>
          ) : undefined
        }
      />
      <div className="page-tabs">
        <button aria-pressed={tab === 'invites'} onClick={() => setTab('invites')}>
          <Ticket size={16} />
          Invite codes<Badge>{data.invites.length}</Badge>
        </button>
        <button aria-pressed={tab === 'rewards'} onClick={() => setTab('rewards')}>
          <Coins size={16} />
          BOOSTERPASS codes<Badge>{data.rewards.length}</Badge>
        </button>
      </div>
      {tab === 'invites' ? (
        <>
          <div className="context-note">
            <Ticket size={19} />
            <div>
              <strong>There’s room for a few more.</strong>
              <p>
                Players enter an invite when creating an account in the launcher. Registration is
                currently{' '}
                <strong>
                  {data.settings.registration === 'invite'
                    ? 'invite-only'
                    : data.settings.registration}
                </strong>
                .
              </p>
            </div>
          </div>
          <Panel>
            {data.invites.length ? (
              <>
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th scope="col">Invite code</th>
                        <th scope="col">Status</th>
                        <th scope="col">Sign-ups</th>
                        <th scope="col">Expires</th>
                        <th scope="col">Created by</th>
                      </tr>
                    </thead>
                    <tbody>
                      {data.invites.map((i) => {
                        const expired = new Date(i.expires).getTime() < now
                        const used = i.uses >= i.limit
                        return (
                          <tr key={i.code}>
                            <td>
                              <div className="code-cell">
                                <code>{i.code}</code>
                                <CopyButton value={i.code} label={`Copy ${i.code}`} />
                              </div>
                            </td>
                            <td>
                              <Badge tone={expired || used ? 'neutral' : 'green'} dot>
                                {expired ? 'Expired' : used ? 'Used up' : 'Active'}
                              </Badge>
                            </td>
                            <td>
                              <strong className="table-emphasis">
                                {i.limit - i.uses} sign-ups left
                              </strong>
                              <small className="cell-subtitle">
                                {i.uses} of {i.limit} used
                              </small>
                            </td>
                            <td className="nowrap muted">{date(i.expires)}</td>
                            <td>{i.by}</td>
                          </tr>
                        )
                      })}
                    </tbody>
                  </table>
                </div>
                <div className="table-footer">
                  <span>
                    {
                      data.invites.filter(
                        (i) => i.uses < i.limit && new Date(i.expires).getTime() > now,
                      ).length
                    }{' '}
                    active invites
                  </span>
                  <span>
                    <CalendarClock size={14} />
                    Expired codes cannot be used
                  </span>
                </div>
              </>
            ) : (
              <EmptyState
                title="No invite codes yet"
                description="Create a code to invite someone to your server."
                action={<Button onClick={() => setInvite(true)}>Create invite</Button>}
              />
            )}
          </Panel>
        </>
      ) : (
        <>
          <div className="context-note">
            <Coins size={19} />
            <div>
              <strong>A few extra BP for the next pack.</strong>
              <p>Each account can redeem a code once in the game’s BOOSTERPASS menu.</p>
            </div>
          </div>
          {role !== 'admin' && (
            <ReadOnlyNote>Only administrators can create or remove BP codes.</ReadOnlyNote>
          )}
          <Panel>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Code</th>
                    <th scope="col">Reward</th>
                    <th scope="col">Claims</th>
                    <th scope="col">Remaining</th>
                    <th scope="col">
                      <span className="sr-only">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {data.rewards.map((r) => (
                    <tr key={r.code}>
                      <td>
                        <div className="code-cell">
                          <code>{r.code}</code>
                          <CopyButton value={r.code} label={`Copy ${r.code}`} />
                        </div>
                      </td>
                      <td>
                        <span className="reward-amount">
                          <Coins size={15} />
                          {number(r.bp)} BP
                        </span>
                      </td>
                      <td>
                        {r.claims} of {r.limit}
                      </td>
                      <td>
                        <Badge tone={r.claims < r.limit ? 'green' : 'neutral'}>
                          {r.limit - r.claims} available
                        </Badge>
                      </td>
                      <td>
                        {role === 'admin' && (
                          <Button
                            variant="ghost"
                            className="icon-button danger-text"
                            aria-label={`Remove ${r.code}`}
                            onClick={() => setRemove(r)}
                          >
                            <Trash2 size={16} />
                          </Button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {!data.rewards.length && (
              <EmptyState
                title="No BP codes yet"
                description="A reward code can help players pick up their next pack."
              />
            )}
          </Panel>
        </>
      )}
      <InviteDialog open={invite} onOpenChange={setInvite} />
      {reward && <RewardDialog open close={() => setReward(false)} />}
      <Modal
        open={remove !== null}
        onOpenChange={(v) => {
          if (!v) setRemove(null)
        }}
        title="Remove this BP code?"
        description={`${remove?.code ?? 'This code'} will no longer be redeemable. Already claimed BP stays with the players.`}
      >
        <div className="modal-actions">
          <Button onClick={() => setRemove(null)}>Cancel</Button>
          <Button
            variant="danger"
            onClick={() => {
              if (remove) {
                change(
                  (s) => ({ ...s, rewards: s.rewards.filter((r) => r.code !== remove.code) }),
                  'Removed a BOOSTERPASS code',
                  remove.code,
                )
                notify('BP code removed.')
                setRemove(null)
              }
            }}
          >
            <Check size={16} />
            Remove code
          </Button>
        </div>
      </Modal>
    </>
  )
}
