import { Coins, Gift, KeyRound, Pencil, Plus, ShieldCheck, Trash2 } from 'lucide-react'
import { Badge, Button, Panel, ReadOnlyNote } from './components/ui'
import type { AccountDetail, AccountEditRequest, StaffSession } from './api/generated/http/models'

type Edit = (mode: AccountEditRequest['kind'], playerId?: number, name?: string) => void
export function AccountDetails({
  data,
  session,
  canStart,
  canRename,
  canGiveBp,
  onEdit,
  onGiveBp,
  onGift,
  onRevealKey,
  onResetPassword,
  onModerate,
  onDelete,
}: {
  data: AccountDetail
  session: StaffSession
  canStart: boolean
  canRename: boolean
  canGiveBp: boolean
  onEdit: Edit
  onGiveBp: () => void
  onGift: (player: { player_id: number; name: string }) => void
  onRevealKey: () => void
  onResetPassword: () => void
  onModerate: () => void
  onDelete: () => void
}) {
  const admin = session.role === 'admin'
  const standing = data.standing?.kind ?? 'Key account'
  const canModerate =
    canStart && (admin || data.role === 'player' || data.user_id === session.user_id)
  return (
    <div className="account-workspace">
      <div className="account-summary-grid">
        <div className="account-stat">
          <span>Account balance</span>
          <strong data-testid="account-balance">{data.bp.toLocaleString()} BP</strong>
          <Button disabled={!canGiveBp} onClick={onGiveBp}>
            <Coins size={16} />
            Give BP
          </Button>
        </div>
        <div className="account-stat">
          <span>Players</span>
          <strong>
            {data.players.length} <small>/ 4 slots</small>
          </strong>
        </div>
        <div className="account-stat">
          <span>Account status</span>
          <Badge
            tone={standing === 'banned' ? 'red' : standing === 'restricted' ? 'amber' : 'green'}
          >
            {standing}
          </Badge>
          <p>
            {data.muted ? 'Chat muted' : 'Chat unrestricted'} ·{' '}
            {data.role === 'admin'
              ? 'Administrator'
              : data.role === 'moderator'
                ? 'Moderator'
                : 'Player'}
          </p>
        </div>
      </div>
      {!admin && (
        <ReadOnlyNote>
          Only administrators can give rewards or change roles, country and game keys.
        </ReadOnlyNote>
      )}
      <div className="account-layout">
        <div className="account-primary">
          <section aria-label="Players" className="account-players">
            <Panel
              title="Players"
              description="Collections, decks and checked game presence."
              action={
                <Button
                  disabled={!admin || !canStart || data.players.length >= 4}
                  onClick={() => onEdit('add_player')}
                  title={data.players.length >= 4 ? 'All four player slots are in use' : undefined}
                >
                  <Plus size={16} />
                  Add player
                </Button>
              }
            >
              <div className="account-player-list">
                {data.players.map((player) => (
                  <article className="account-player" key={player.player_id}>
                    <header>
                      <div>
                        <h3>{player.name}</h3>
                        <p>
                          Player #{player.player_id} · Slot {player.slot + 1} · Level {player.level}
                        </p>
                      </div>
                      <Badge tone={player.online ? 'green' : 'neutral'}>
                        {player.online == null
                          ? 'Presence unknown'
                          : player.online
                            ? 'Online'
                            : 'Offline'}
                      </Badge>
                    </header>
                    <dl className="profile-facts">
                      <div>
                        <dt>Additional cards</dt>
                        <dd>
                          {player.additional_cards.toLocaleString()}{' '}
                          <small className="block muted">{player.card_kinds} kinds</small>
                        </dd>
                      </div>
                      <div>
                        <dt>Items owned</dt>
                        <dd>{player.owned_items.toLocaleString()}</dd>
                      </div>
                      <div>
                        <dt>Decks</dt>
                        <dd>
                          {player.purchased_decks} purchased · {player.uploaded_decks} uploaded
                        </dd>
                      </div>
                      <div>
                        <dt>Duel record</dt>
                        <dd>
                          {player.record
                            ? `${player.record.wins} W / ${player.record.losses} L / ${player.record.draws} D`
                            : 'Unavailable'}
                        </dd>
                      </div>
                      <div>
                        <dt>Title</dt>
                        <dd>#{player.title_id}</dd>
                      </div>
                    </dl>
                    <div className="account-player-actions">
                      <Button
                        aria-label={`Rename ${player.name}`}
                        disabled={!canRename || !canStart}
                        onClick={() => onEdit('rename_player', player.player_id, player.name)}
                      >
                        <Pencil size={15} />
                        Rename
                      </Button>
                      <Button
                        aria-label={`Gift to ${player.name}`}
                        disabled={!admin || !canStart}
                        onClick={() => onGift({ player_id: player.player_id, name: player.name })}
                      >
                        <Gift size={15} />
                        Give a gift
                      </Button>
                    </div>
                  </article>
                ))}
                {!data.players.length && (
                  <div className="connected-empty">
                    <p>No players yet. Add a player to start their collection.</p>
                  </div>
                )}
              </div>
              <p className="account-section-note">
                Additional cards exclude the starter deck. Presence is a snapshot; refresh for the
                latest state.
              </p>
            </Panel>
          </section>
          <section aria-label="Moderation">
            <Panel
              title="Moderation"
              description="Manage game access, chat restrictions and active sessions."
            >
              <div className="account-moderation">
                <dl className="profile-facts">
                  <div>
                    <dt>Game access</dt>
                    <dd>
                      {standing}
                      {data.standing && data.standing.kind !== 'active' && (
                        <>
                          <p>{data.standing.reason}</p>
                          <small className="muted">
                            {data.standing.until
                              ? `Until ${new Date(data.standing.until).toLocaleString()}`
                              : 'Until lifted'}
                          </small>
                        </>
                      )}
                    </dd>
                  </div>
                  <div>
                    <dt>Chat</dt>
                    <dd>
                      {data.muted ? (
                        <>
                          {data.muted.reason || 'Muted'}
                          <small className="block muted">
                            {data.muted.until
                              ? `Until ${new Date(data.muted.until).toLocaleString()}`
                              : 'Until lifted'}
                          </small>
                        </>
                      ) : (
                        'Not muted'
                      )}
                    </dd>
                  </div>
                </dl>
                <Button disabled={!canModerate} onClick={onModerate}>
                  <ShieldCheck size={16} />
                  Moderate account
                </Button>
              </div>
            </Panel>
          </section>
        </div>
        <div className="account-side">
          <section aria-label="Account access">
            <Panel title="Account access" description="Staff permissions and sign-in credentials.">
              <div className="account-control-list">
                <div>
                  <div>
                    <strong>Role</strong>
                    <p>
                      {data.role === 'admin'
                        ? 'Administrator'
                        : data.role === 'moderator'
                          ? 'Moderator'
                          : 'Player'}
                    </p>
                  </div>
                  <Button
                    disabled={!admin || !data.login || data.password_ready === false || !canStart}
                    onClick={() => onEdit('set_role')}
                    title={
                      !data.login
                        ? 'Only password accounts can have staff roles'
                        : data.password_ready === false
                          ? 'Finish password setup first'
                          : undefined
                    }
                  >
                    Change role
                  </Button>
                </div>
                <div>
                  <div>
                    <strong>Sign-in method</strong>
                    <p>
                      {data.login
                        ? data.password_ready === false
                          ? 'Password setup required'
                          : 'User ID and password'
                        : data.has_key
                          ? 'Game key'
                          : 'Not configured'}
                    </p>
                  </div>
                </div>
                <div>
                  <div>
                    <strong>Game key</strong>
                    <p>{data.has_key ? 'Key assigned' : 'No key assigned'}</p>
                  </div>
                  <Button disabled={!admin || !canStart} onClick={onRevealKey}>
                    <KeyRound size={15} />
                    Reveal key
                  </Button>
                </div>
                <div>
                  <div>
                    <strong>Password recovery</strong>
                    <p>
                      {!data.login
                        ? 'Key accounts do not have passwords.'
                        : data.reset_pending
                          ? 'Recovery code pending'
                          : 'No pending code'}
                    </p>
                  </div>
                  <Button
                    disabled={!data.login || !canRename || !canStart}
                    onClick={onResetPassword}
                  >
                    Reset password
                  </Button>
                </div>
                <div>
                  <div>
                    <strong>Remembered devices</strong>
                    <p>
                      {data.device_count} saved {data.device_count === 1 ? 'device' : 'devices'}
                    </p>
                  </div>
                </div>
              </div>
            </Panel>
          </section>
          <section aria-label="Profile">
            <Panel title="Profile">
              <div className="account-control-list">
                <div>
                  <div>
                    <strong>Country</strong>
                    <p>{data.country ?? 'Automatic (game language)'}</p>
                  </div>
                  <Button disabled={!admin || !canStart} onClick={() => onEdit('set_country')}>
                    Change country
                  </Button>
                </div>
              </div>
            </Panel>
          </section>
        </div>
      </div>
      {admin && (
        <section aria-label="Delete account" className="account-danger">
          <div>
            <h2>Delete account</h2>
            <p>
              Permanently remove this account and its saved game data. You will review and confirm
              the deletion first.
            </p>
          </div>
          <Button variant="ghost" className="danger-text" disabled={!canStart} onClick={onDelete}>
            <Trash2 size={16} />
            Delete account…
          </Button>
        </section>
      )}
    </div>
  )
}
