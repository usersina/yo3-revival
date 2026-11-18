import { useDeferredValue, useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { BookOpen, ExternalLink, Gift, Layers, LoaderCircle, RefreshCw } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal } from './components/ui'
import {
  getGetAccountQueryKey,
  getGetStaffSessionQueryKey,
  getListAccountsQueryKey,
  useGivePlayerGift,
  useListGiftChoices,
} from './api/generated/http/client'
import type { GivePlayerGiftMutationVariables } from './api/generated/http/client'
import type { GiftRequest, GiftResult, StaffSession } from './api/generated/http/models'
import {
  GiftRequest as RequestSchema,
  GivePlayerGiftHeader,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

export type GiftTarget = { player_id: number; name: string }
const pendingSchema = z.object({ data: RequestSchema, headers: GivePlayerGiftHeader })
const errorText = (e: unknown) =>
  e instanceof ApiHttpError ? e.message : 'Could not reach the server. Retry the same request.'
export function ConnectedGifts({
  session,
  target,
  onChange,
}: {
  session: StaffSession
  target: GiftTarget | null
  onChange: (value: GiftTarget | null) => void
}) {
  const client = useQueryClient()
  const storageKey = `yo3:pending-gift:${session.user_id}`
  const [initial] = useState(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { pending: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: '' }
    } catch {
      return {
        pending: null,
        error: 'Could not restore a gift request. Check the audit log before giving another gift.',
      }
    }
  })
  const [pending, setPending] = useState<GivePlayerGiftMutationVariables | null>(initial.pending)
  const [storageError, setStorageError] = useState(initial.error)
  const [kind, setKind] = useState<GiftRequest['kind']>('cards')
  const [query, setQuery] = useState('')
  const search = useDeferredValue(query)
  const [choice, setChoice] = useState('')
  const [copies, setCopies] = useState('1')
  const [includeCards, setIncludeCards] = useState(false)
  const [accepted, setAccepted] = useState(false)
  const [result, setResult] = useState<GiftResult | null>(null)
  const [failure, setFailure] = useState('')
  const [refused, setRefused] = useState(false)
  const choices = useListGiftChoices(
    { q: search },
    { query: { enabled: target !== null && !pending && !result && session.role === 'admin' } },
  )
  const mutation = useGivePlayerGift({
    mutation: {
      onSuccess: (value) => {
        setResult(value)
        setFailure('')
        setRefused(false)
        try {
          sessionStorage.removeItem(storageKey)
          setPending(null)
        } catch {
          setStorageError('The gift was saved, but this browser could not clear its retry request.')
        }
        void client.invalidateQueries({ queryKey: getListAccountsQueryKey() })
        void client.invalidateQueries({ queryKey: getGetAccountQueryKey(value.user_id) })
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
  const playerId = pending?.data.player_id ?? target?.player_id
  const catalogueBusy = choices.isFetching || query !== search
  const selectedCard =
    kind === 'cards' && !catalogueBusy && !choices.error
      ? choices.data?.cards.find((card) => String(card.row) === choice)
      : undefined
  const validCopies =
    kind !== 'cards' ||
    (Number.isInteger(Number(copies)) && Number(copies) >= 1 && Number(copies) <= 255)
  const options =
    kind === 'cards'
      ? choices.data?.cards.map((c) => ({
          id: c.row,
          label: `${c.name} · Art ${c.art} · Konami ${c.konami_id}`,
        }))
      : kind === 'recipe'
        ? choices.data?.recipes.map((r) => ({
            id: r.deck_id,
            label: `${r.name} · ${r.card_count} cards`,
          }))
        : choices.data?.titles.map((t) => ({ id: t.title_id, label: t.name }))
  const more =
    kind === 'cards'
      ? choices.data?.more_cards
      : kind === 'recipe'
        ? choices.data?.more_recipes
        : choices.data?.more_titles
  function submit(event: FormEvent) {
    event.preventDefault()
    if (mutation.isPending || result || storageError) return
    let variables = pending
    if (!variables) {
      if (
        !playerId ||
        !accepted ||
        !validCopies ||
        !options?.some((o) => String(o.id) === choice) ||
        catalogueBusy ||
        choices.error
      )
        return
      const data: GiftRequest =
        kind === 'cards'
          ? { kind, player_id: playerId, row: Number(choice), count: Number(copies) }
          : kind === 'recipe'
            ? { kind, player_id: playerId, deck_id: Number(choice), include_cards: includeCards }
            : { kind, player_id: playerId, title_id: Number(choice) }
      variables = { data, headers: { 'Idempotency-Key': crypto.randomUUID() } }
      if (!pendingSchema.safeParse(variables).success) {
        setFailure('Choose a gift and 1–255 copies.')
        return
      }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(variables))
        setPending(variables)
      } catch {
        setStorageError('Could not retain this gift safely. No request was sent.')
        return
      }
    }
    setFailure('')
    mutation.mutate(variables)
  }
  function close() {
    if (mutation.isPending) return
    setResult(null)
    mutation.reset()
    setAccepted(false)
    onChange(null)
    if (!pending || result) {
      setFailure('')
      setRefused(false)
      setChoice('')
      setQuery('')
      setCopies('1')
      setIncludeCards(false)
    }
  }
  function edit() {
    try {
      sessionStorage.removeItem(storageKey)
      if (pending) {
        const data = pending.data
        setKind(data.kind)
        setChoice(
          String(
            data.kind === 'cards'
              ? data.row
              : data.kind === 'recipe'
                ? data.deck_id
                : data.title_id,
          ),
        )
        if (data.kind === 'cards') setCopies(String(data.count))
        if (data.kind === 'recipe') setIncludeCards(data.include_cards)
      }
      setPending(null)
      setFailure('')
      setRefused(false)
      setAccepted(false)
    } catch {
      setStorageError('Could not clear this request. No new gift will be sent.')
    }
  }
  const outcome = result?.gift
  return (
    <>
      {pending && (
        <Button
          onClick={() =>
            onChange({
              player_id: pending.data.player_id,
              name: `Player #${pending.data.player_id}`,
            })
          }
          disabled={session.role !== 'admin'}
        >
          <Gift size={15} /> Resume gift
        </Button>
      )}
      {storageError && (
        <p role="alert" className="connected-error">
          {storageError}
        </p>
      )}
      <Modal
        open={target !== null}
        onOpenChange={(open) => {
          if (!open) close()
        }}
        title="Give a gift"
        description={`${result ? `Player #${result.player_id}` : pending ? `Player #${playerId}` : (target?.name ?? '')} · Saved gifts are audited.`}
      >
        <form id="connected-gift" className="modal-body" onSubmit={submit}>
          {result ? (
            <div role="status" className="inline-notice notice-success">
              <div>
                <strong>Gift confirmed for player #{result.player_id}</strong>
                <p>
                  {outcome?.kind === 'cards'
                    ? `${outcome.added} cards saved. Additional copies of artwork #${outcome.row}: ${outcome.total}`
                    : outcome?.kind === 'recipe'
                      ? `Recipe #${outcome.deck_id} saved ${outcome.include_cards ? 'with' : 'without'} its cards.`
                      : outcome?.kind === 'title'
                        ? outcome.already_owned
                          ? `This player already owns title #${outcome.title_id}.`
                          : `Title #${outcome.title_id} saved.`
                        : ''}
                </p>
                <p className="field-hint">
                  Cards appear at the next map visit. Recipes appear after the next login or duel;
                  titles in Avatar Settings. Cards and recipes include D-Mail.
                </p>
              </div>
            </div>
          ) : pending ? (
            <div className="inline-notice notice-warning">
              <div>
                <strong>Unconfirmed gift for player #{pending.data.player_id}</strong>
                <p>
                  The gift may already be saved. Retry this request to confirm it without giving it
                  twice.
                </p>
                <p className="field-hint">
                  {pending.data.kind === 'cards'
                    ? `${pending.data.count} copies of artwork #${pending.data.row}`
                    : pending.data.kind === 'recipe'
                      ? `Recipe #${pending.data.deck_id}${pending.data.include_cards ? ' with its cards' : ' without its cards'}`
                      : `Title #${pending.data.title_id}`}
                </p>
              </div>
            </div>
          ) : (
            <>
              <Field id="gift-kind" label="Gift type">
                <select
                  id="gift-kind"
                  value={kind}
                  onChange={(e) => {
                    setKind(e.target.value as GiftRequest['kind'])
                    setChoice('')
                    setAccepted(false)
                  }}
                >
                  <option value="cards">Cards</option>
                  <option value="recipe">Deck recipe</option>
                  <option value="title">Title</option>
                </select>
              </Field>
              <Field
                id="gift-search"
                label="Find a gift"
                hint="Search by English name or ID. Choices come from the host’s game files."
              >
                <input
                  id="gift-search"
                  type="search"
                  placeholder={kind === 'cards' ? 'Card name or Konami ID…' : 'Name or ID…'}
                  aria-describedby="gift-search-hint"
                  value={query}
                  maxLength={80}
                  onChange={(e) => {
                    setQuery(e.target.value)
                    setChoice('')
                    setAccepted(false)
                  }}
                  autoComplete="off"
                />
              </Field>
              {catalogueBusy && (
                <p role="status">
                  <LoaderCircle size={15} className="animate-spin" /> Loading gift choices…
                </p>
              )}
              {choices.error ? (
                <div role="alert" className="connected-error">
                  {errorText(choices.error)}
                  <Button onClick={() => void choices.refetch()} disabled={choices.isFetching}>
                    <RefreshCw size={14} /> Retry choices
                  </Button>
                </div>
              ) : kind === 'cards' ? (
                <fieldset className="gift-artworks" disabled={catalogueBusy}>
                  <legend>Card artwork</legend>
                  <div className="gift-options">
                    {choices.data?.cards.map((card) => (
                      <label className="gift-option" key={card.row}>
                        <input
                          type="radio"
                          name="gift-card"
                          value={card.row}
                          checked={String(card.row) === choice}
                          aria-label={`${card.name} · Artwork ${card.art} · Konami ${card.konami_id}`}
                          onChange={() => {
                            setChoice(String(card.row))
                            setAccepted(false)
                          }}
                        />
                        <span className="gift-symbol" aria-hidden="true">
                          <Layers size={21} />
                        </span>
                        <span className="gift-card-name">
                          <strong>{card.name}</strong>
                          <small>
                            Artwork {card.art} · Konami {card.konami_id}
                          </small>
                        </span>
                      </label>
                    ))}
                  </div>
                  {choices.data?.cards.length === 0 && (
                    <p className="field-hint">No matching gifts. Try another name or ID.</p>
                  )}
                  {more && (
                    <p className="field-hint">
                      Showing the first 50. Search to narrow the choices.
                    </p>
                  )}
                </fieldset>
              ) : (
                <Field id="gift-choice" label="Gift">
                  <select
                    id="gift-choice"
                    value={choice}
                    disabled={catalogueBusy || !options?.length}
                    onChange={(e) => {
                      setChoice(e.target.value)
                      setAccepted(false)
                    }}
                  >
                    <option value="">Choose a gift…</option>
                    {options?.map((o) => (
                      <option key={o.id} value={o.id}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  {options?.length === 0 && (
                    <p className="field-hint">No matching gifts. Try another name or ID.</p>
                  )}
                  {more && (
                    <p className="field-hint">
                      Showing the first 50. Search to narrow the choices.
                    </p>
                  )}
                </Field>
              )}
              {selectedCard && (
                <div className="gift-selection" aria-label="Selected card">
                  <strong>
                    {selectedCard.name} · Artwork {selectedCard.art}
                  </strong>
                  {selectedCard.konami_id > 0 && (
                    <a
                      href={`https://www.db.yugioh-card.com/yugiohdb/card_search.action?ope=2&cid=${selectedCard.konami_id}&request_locale=en`}
                      target="_blank"
                      rel="noopener noreferrer"
                      aria-label="Card details (opens in a new tab)"
                    >
                      <BookOpen size={15} /> Card details <ExternalLink size={13} />
                    </a>
                  )}
                  <p className="field-hint">
                    Official database with current text. In-game effects may follow older card text.
                  </p>
                </div>
              )}
              {kind === 'cards' && (
                <Field
                  id="gift-count"
                  label="Copies"
                  hint="1–255 additional copies per artwork in total."
                >
                  <input
                    id="gift-count"
                    type="number"
                    min={1}
                    max={255}
                    step={1}
                    required
                    aria-invalid={!validCopies}
                    aria-describedby="gift-count-hint"
                    value={copies}
                    onChange={(e) => {
                      setCopies(e.target.value)
                      setAccepted(false)
                    }}
                  />
                </Field>
              )}
              {kind === 'recipe' && (
                <label className="profile-role-confirm">
                  <input
                    type="checkbox"
                    checked={includeCards}
                    onChange={(e) => {
                      setIncludeCards(e.target.checked)
                      setAccepted(false)
                    }}
                  />{' '}
                  Include the recipe’s cards
                </label>
              )}
            </>
          )}
          {failure && (
            <div role="alert" className="connected-error">
              <p>{failure}</p>
              {pending && !refused && <p>Retry the same request to confirm the outcome.</p>}
            </div>
          )}
          {pending && (
            <p className="field-hint">Request ID: {pending.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions gift-actions">
          {!result && !pending && (
            <label className="profile-role-confirm">
              <input
                type="checkbox"
                form="connected-gift"
                checked={accepted}
                onChange={(e) => setAccepted(e.target.checked)}
              />{' '}
              Confirm this gift for the selected player
            </label>
          )}
          <Button onClick={close} disabled={mutation.isPending}>
            {result ? 'Done' : 'Close'}
          </Button>
          {pending && refused && <Button onClick={edit}>Edit request</Button>}
          {!result && (
            <Button
              variant="primary"
              type="submit"
              form="connected-gift"
              disabled={
                mutation.isPending ||
                !!storageError ||
                refused ||
                (!pending &&
                  (!accepted || !choice || !validCopies || catalogueBusy || !!choices.error))
              }
            >
              {mutation.isPending && <LoaderCircle size={15} className="animate-spin" />}
              {mutation.isPending ? 'Confirming…' : pending ? 'Retry same request' : 'Give gift'}
            </Button>
          )}
        </div>
      </Modal>
    </>
  )
}
