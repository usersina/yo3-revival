import { useState } from 'react'
import { Ticket } from 'lucide-react'
import { usePreview } from '../preview'
import { Button, Field, Modal } from './ui'

export function InviteDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { change, notify, role } = usePreview()
  const [uses, setUses] = useState('1')
  const [days, setDays] = useState('7')
  const valid = /^\d+$/.test(uses) && +uses >= 1 && +uses <= 1000 && ['1', '7', '30'].includes(days)
  function create(e: React.FormEvent) {
    e.preventDefault()
    if (!valid) return
    const bytes = crypto.getRandomValues(new Uint8Array(6))
    const suffix = Array.from(bytes, (b) => 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'[b % 32]).join('')
    const code = `YO3-${suffix}`
    const expires = new Date(Date.now() + +days * 86_400_000).toISOString()
    change(
      (s) => ({
        ...s,
        invites: [
          {
            code,
            uses: 0,
            limit: +uses,
            expires,
            by: role === 'admin' ? 'TestAdmin' : 'TestModerator',
          },
          ...s.invites,
        ],
      }),
      'Created an invite',
      code,
    )
    notify('Invite created in the preview.')
    onOpenChange(false)
  }
  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title="Create invite"
      description="Give someone a place at the table. They'll enter this code when creating an account."
    >
      <form onSubmit={create}>
        <div className="modal-body">
          <div className="form-grid">
            <Field
              id="invite-uses"
              label="Number of sign-ups"
              hint="How many accounts can use this code."
            >
              <input
                id="invite-uses"
                type="number"
                min="1"
                max="1000"
                required
                value={uses}
                onChange={(e) => setUses(e.target.value)}
              />
            </Field>
            <Field id="invite-days" label="Expires after">
              <select id="invite-days" value={days} onChange={(e) => setDays(e.target.value)}>
                <option value="1">1 day</option>
                <option value="7">7 days</option>
                <option value="30">30 days</option>
              </select>
            </Field>
          </div>
        </div>
        <div className="modal-actions">
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" type="submit" disabled={!valid}>
            <Ticket size={16} />
            Create invite
          </Button>
        </div>
      </form>
    </Modal>
  )
}
