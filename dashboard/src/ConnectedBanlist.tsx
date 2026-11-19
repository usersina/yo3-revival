import { useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { Button, Field, Panel } from './components/ui'
import { SelectControl, SelectOption } from './components/form-controls'
import {
  getGetAccountQueryKey,
  useListBanlists,
  useSetAccountBanlist,
} from './api/generated/http/client'
import type { AccountDetail } from './api/generated/http/models'
import { ApiHttpError } from './api/transport'

const SERVER_DEFAULT = 'default'
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry.'

/** The banlist an account's game logs in with, and the staff's override. Saving again is harmless. */
export function ConnectedBanlist({ account }: { account: AccountDetail }) {
  const client = useQueryClient()
  const choices = useListBanlists()
  const [picked, setPicked] = useState<string | null>(null)
  const [notice, setNotice] = useState('')
  const [requestId, setRequestId] = useState(() => crypto.randomUUID())
  const current = account.banlist.chosen ? String(account.banlist.id) : SERVER_DEFAULT
  const value = picked ?? current
  const save = useSetAccountBanlist({
    mutation: {
      onSuccess: (result) => {
        setNotice(
          result.chosen
            ? `Saved: ${result.name} from the player's next login.`
            : `Saved: the server's default (${result.name}) from the player's next login.`,
        )
        setPicked(null)
        setRequestId(crypto.randomUUID())
        void client.invalidateQueries({ queryKey: getGetAccountQueryKey(account.user_id) })
      },
    },
  })
  return (
    <div role="region" aria-label="Banlist">
      <Panel
        title="Banlist"
        description="The banlist the game checks decks against for Auto Duel and computer duels."
      >
        <p>
          <strong>{account.banlist.name}</strong>{' '}
          <span className="muted">
            {account.banlist.chosen ? '(chosen by the player or staff)' : "(the server's default)"}
          </span>
        </p>
        <Field
          id="account-banlist"
          label="Banlist for next login"
          hint="Players choose their own with /banlist in the game's chat. A change applies at the player's next login."
        >
          <SelectControl
            id="account-banlist"
            value={value}
            onValueChange={(next) => {
              setPicked(next)
              setNotice('')
            }}
            disabled={save.isPending || !choices.data}
          >
            <SelectOption value={SERVER_DEFAULT}>
              {choices.data
                ? `Server default (${choices.data.server_default.name})`
                : 'Server default'}
            </SelectOption>
            {choices.data?.choices.map((choice) => (
              <SelectOption key={choice.id} value={String(choice.id)}>
                {choice.name}
              </SelectOption>
            ))}
          </SelectControl>
        </Field>
        {choices.isPending && <p role="status">Loading banlists…</p>}
        {choices.error && (
          <div role="alert" className="connected-error">
            {errorText(choices.error)}
            <Button onClick={() => void choices.refetch()}>Retry banlists</Button>
          </div>
        )}
        {save.error && (
          <p role="alert" className="connected-error">
            {errorText(save.error)}
          </p>
        )}
        {notice && (
          <p className="inline-notice notice-success" role="status">
            {notice}
          </p>
        )}
        <Button
          variant="primary"
          disabled={save.isPending || picked === null || picked === current}
          onClick={() =>
            save.mutate({
              data: {
                user_id: account.user_id,
                banlist: value === SERVER_DEFAULT ? null : Number(value),
              },
              headers: { 'Idempotency-Key': requestId },
            })
          }
        >
          {save.isPending && <LoaderCircle size={16} className="animate-spin" />}
          Apply banlist
        </Button>
      </Panel>
    </div>
  )
}
