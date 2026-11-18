import { Input } from './components/form-controls'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Clipboard, LoaderCircle, Plus, RefreshCw, Trash2 } from 'lucide-react'
import { z } from 'zod'
import { Button, Field, Modal, Panel, ReadOnlyNote } from './components/ui'
import {
  getGetStaffSessionQueryKey,
  getListBpCodesQueryKey,
  useCreateBpCode,
  useListBpCodes,
  useRemoveBpCode,
} from './api/generated/http/client'
import type {
  CreateBpCodeMutationVariables,
  RemoveBpCodeMutationVariables,
} from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import {
  CreateBpCodeHeader,
  CreateBpCodeRequest,
  RemoveBpCodeHeader,
  RemoveBpCodeRequest,
} from './api/generated/validation/schemas'
import { ApiHttpError } from './api/transport'

type Intent =
  | ({ kind: 'create' } & CreateBpCodeMutationVariables)
  | ({ kind: 'remove' } & RemoveBpCodeMutationVariables)
const pendingSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('create'), data: CreateBpCodeRequest, headers: CreateBpCodeHeader }),
  z.object({ kind: z.literal('remove'), data: RemoveBpCodeRequest, headers: RemoveBpCodeHeader }),
])
const errorText = (error: unknown) =>
  error instanceof ApiHttpError
    ? error.message
    : 'Could not reach the server. Check the connection and retry this same change.'

export function ConnectedCodes({ session }: { session: StaffSession }) {
  const client = useQueryClient()
  const codes = useListBpCodes()
  const storageKey = `yo3:pending-code:${session.user_id}`
  const [stored, setStored] = useState<{ intent: Intent | null; error: string | null }>(() => {
    try {
      const raw = sessionStorage.getItem(storageKey)
      return { intent: raw ? pendingSchema.parse(JSON.parse(raw)) : null, error: null }
    } catch {
      return {
        intent: null,
        error:
          'Could not restore a saved code change. Review the server audit log before making another.',
      }
    }
  })
  const [dialog, setDialog] = useState<'create' | 'remove' | null>(null)
  const [custom, setCustom] = useState('')
  const [bp, setBp] = useState('500')
  const [uses, setUses] = useState('20')
  const [selected, setSelected] = useState('')
  const [notice, setNotice] = useState('')
  const [failure, setFailure] = useState('')
  const [uncertain, setUncertain] = useState(false)

  function failed(error: unknown) {
    const refused =
      error instanceof ApiHttpError && [400, 404, 409, 413, 415].includes(error.status)
    setUncertain(!refused)
    setFailure(errorText(error))
    if (refused) {
      try {
        sessionStorage.removeItem(storageKey)
        setStored({ intent: null, error: null })
      } catch {
        setStored((previous) => ({
          ...previous,
          error: 'Could not clear the refused request. Reload before making another code change.',
        }))
      }
    }
    if (error instanceof ApiHttpError && [401, 403].includes(error.status)) {
      void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
    }
  }
  function completed(message: string) {
    try {
      sessionStorage.removeItem(storageKey)
      setStored({ intent: null, error: null })
    } catch {
      setStored((previous) => ({
        ...previous,
        error:
          'Saved, but this browser could not clear the request. Reload to confirm the same change safely.',
      }))
    }
    setNotice(message)
    setFailure('')
    setDialog(null)
    void client.invalidateQueries({ queryKey: getListBpCodesQueryKey() })
  }
  const create = useCreateBpCode({
    mutation: {
      onError: failed,
      onSuccess: (result) => completed(`Code created: ${result.code}. Saved with its audit entry.`),
    },
  })
  const remove = useRemoveBpCode({
    mutation: {
      onError: failed,
      onSuccess: (result) =>
        completed(`Code removed: ${result.removed}. Already claimed BP stays with the players.`),
    },
  })
  const busy = create.isPending || remove.isPending
  const admin = session.role === 'admin'
  const canStart = admin && !busy && !stored.intent && !stored.error
  const draft = CreateBpCodeRequest.safeParse({
    code: custom.trim() || null,
    bp: Number(bp),
    uses: uses.trim() ? Number(uses) : null,
  })

  function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || !admin || stored.error) return
    let intent = stored.intent
    if (!intent) {
      const headers = { 'Idempotency-Key': crypto.randomUUID() }
      if (dialog === 'create') {
        if (!draft.success) return
        intent = { kind: 'create', data: draft.data, headers }
      } else {
        if (!RemoveBpCodeRequest.safeParse({ code: selected }).success) return
        intent = { kind: 'remove', data: { code: selected }, headers }
      }
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(intent))
      } catch {
        setStored({
          intent: null,
          error:
            'This browser could not retain the request ID. Enable session storage before changing codes.',
        })
        return
      }
      setStored({ intent, error: null })
    }
    setFailure('')
    if (intent.kind === 'create') create.mutate(intent)
    else remove.mutate(intent)
  }
  function open(kind: 'create' | 'remove', code = '') {
    setSelected(code)
    setCustom('')
    setBp('500')
    setUses('20')
    setNotice('')
    setFailure('')
    setDialog(kind)
  }

  return (
    <section aria-label="BOOSTERPASS codes" className="connected-codes">
      <Panel
        title="BOOSTERPASS codes"
        description="Players enter these in the game's BOOSTERPASS menu. Each account can claim a code once."
        action={
          <Button
            variant="primary"
            onClick={() => open('create')}
            disabled={!canStart}
            title={!admin ? 'Administrators only' : undefined}
          >
            <Plus size={16} />
            Create BP code
          </Button>
        }
      >
        {!admin && <ReadOnlyNote>Only administrators can create or remove codes.</ReadOnlyNote>}
        {notice && (
          <p className="inline-notice notice-success" role="status">
            {notice}
          </p>
        )}
        {stored.error && (
          <p className="connected-error" role="alert">
            {stored.error}
          </p>
        )}
        {stored.intent && (
          <div className="inline-notice">
            <div>
              <strong>Unconfirmed code change</strong>
              <p>It may already be saved. Resume the same request before making another change.</p>
            </div>
            <Button disabled={busy} onClick={() => setDialog(stored.intent!.kind)}>
              Resume code change
            </Button>
          </div>
        )}
        <div className="table-toolbar">
          <span className="muted">
            {codes.data ? `${codes.data.codes.length} codes` : 'Code list'}
          </span>
          <Button onClick={() => void codes.refetch()} disabled={codes.isFetching}>
            <RefreshCw size={15} />
            Refresh codes
          </Button>
        </div>
        {codes.isPending && (
          <p role="status" className="connected-state">
            <LoaderCircle size={17} className="animate-spin" />
            Loading codes…
          </p>
        )}
        {codes.error && (
          <p role="alert" className="connected-error">
            The code list could not be refreshed. {errorText(codes.error)}
            {codes.error instanceof ApiHttpError && codes.error.status === 401 && (
              <Button
                onClick={() =>
                  void client.invalidateQueries({ queryKey: getGetStaffSessionQueryKey() })
                }
              >
                Sign in again
              </Button>
            )}
          </p>
        )}
        {codes.data && (
          <div
            className="table-scroll"
            role="region"
            aria-label="BOOSTERPASS codes table"
            tabIndex={0}
          >
            <table className="responsive-table">
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
                {codes.data.codes.map((code) => (
                  <tr key={code.code}>
                    <td className="row-heading" data-label="Code">
                      <div className="code-cell">
                        <code>{code.code}</code>
                        <Button
                          variant="ghost"
                          className="icon-button"
                          aria-label={`Copy ${code.code}`}
                          onClick={async () => {
                            try {
                              await navigator.clipboard.writeText(code.code)
                              setNotice('Code copied.')
                            } catch {
                              setNotice('Could not copy. Select the code and copy it manually.')
                            }
                          }}
                        >
                          <Clipboard size={15} />
                        </Button>
                      </div>
                    </td>
                    <td className="numeric" data-label="Reward">
                      {code.bp.toLocaleString()} BP
                    </td>
                    <td data-label="Claims">
                      {code.claims.length ? (
                        <details>
                          <summary>{code.claims.length} claims</summary>
                          <ul>
                            {code.claims.map((claim) => (
                              <li key={claim.user_id}>
                                {claim.players.join(', ') || `Account #${claim.user_id}`}
                                {claim.at
                                  ? ` · ${new Date(claim.at).toLocaleString()}`
                                  : ' · Time unavailable'}
                              </li>
                            ))}
                          </ul>
                        </details>
                      ) : (
                        '0'
                      )}
                    </td>
                    <td data-label="Remaining">
                      {code.uses == null
                        ? 'Unlimited'
                        : Math.max(0, code.uses - code.claims.length).toLocaleString()}
                    </td>
                    <td className="row-actions">
                      {admin && (
                        <Button
                          variant="ghost"
                          className="icon-button danger-text"
                          aria-label={`Remove ${code.code}`}
                          disabled={!canStart}
                          onClick={() => open('remove', code.code)}
                        >
                          <Trash2 size={16} />
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {codes.data.codes.length === 0 && (
              <p className="connected-state">No BOOSTERPASS codes yet.</p>
            )}
          </div>
        )}
      </Panel>
      <Modal
        open={dialog !== null}
        onOpenChange={(value) => {
          if (!value && !busy) setDialog(null)
        }}
        title={dialog === 'remove' ? 'Remove this BP code?' : 'Create a BP code'}
        description={
          dialog === 'remove'
            ? `${stored.intent?.data.code ?? selected} will no longer be redeemable. Already claimed BP stays with the players.`
            : 'Create a reward code for players. Saved with an audit entry.'
        }
      >
        <form id="connected-code" onSubmit={submit} className="modal-body">
          {dialog === 'create' && (
            <>
              <Field
                id="code-custom"
                label="Custom code (optional)"
                hint="16 letters or digits. Leave empty to generate a code."
              >
                <Input
                  id="code-custom"
                  value={stored.intent?.data.code ?? custom}
                  onChange={(e) => setCustom(e.target.value.toUpperCase())}
                  maxLength={16}
                  pattern="[A-Za-z0-9]{16}"
                  disabled={busy || Boolean(stored.intent)}
                />
              </Field>
              <Field id="code-bp" label="BP reward">
                <Input
                  id="code-bp"
                  type="number"
                  min={1}
                  max={4294967295}
                  step={1}
                  required
                  value={stored.intent?.kind === 'create' ? stored.intent.data.bp : bp}
                  onChange={(e) => setBp(e.target.value)}
                  disabled={busy || Boolean(stored.intent)}
                />
              </Field>
              <Field
                id="code-uses"
                label="Maximum claims"
                hint="Leave empty for unlimited accounts. Each account can claim it once."
              >
                <Input
                  id="code-uses"
                  type="number"
                  min={1}
                  max={4294967295}
                  step={1}
                  value={stored.intent?.kind === 'create' ? (stored.intent.data.uses ?? '') : uses}
                  onChange={(e) => setUses(e.target.value)}
                  disabled={busy || Boolean(stored.intent)}
                />
              </Field>
            </>
          )}
          {failure && (
            <div role="alert" className="connected-error">
              <p>
                {uncertain
                  ? 'We could not confirm this code change.'
                  : 'The code change was refused.'}
              </p>
              <p>{failure}</p>
              {stored.intent && <p>Retry this same request to avoid a second change.</p>}
            </div>
          )}
          {stored.intent && (
            <p className="field-hint">Request ID: {stored.intent.headers['Idempotency-Key']}</p>
          )}
        </form>
        <div className="modal-actions">
          <span className="muted">
            {stored.intent ? 'Request saved for safe retry' : 'Administrators only'}
          </span>
          <div className="action-group">
            <Button disabled={busy} onClick={() => setDialog(null)}>
              Close
            </Button>
            <Button
              variant={dialog === 'remove' ? 'danger' : 'primary'}
              type="submit"
              form="connected-code"
              disabled={
                busy ||
                !admin ||
                Boolean(stored.error) ||
                (!stored.intent && dialog === 'create' && !draft.success)
              }
            >
              {busy && <LoaderCircle size={16} className="animate-spin" />}
              {busy
                ? 'Saving…'
                : stored.intent
                  ? 'Retry same change'
                  : dialog === 'create'
                    ? 'Create code'
                    : 'Remove code'}
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  )
}
