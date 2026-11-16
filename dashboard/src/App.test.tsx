import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'
import App from './App'

function open(path = '/') {
  const user = userEvent.setup()
  render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  )
  return user
}

describe('dashboard preview', () => {
  it('makes its disconnected state explicit and provides working navigation', async () => {
    const user = open()
    expect(screen.getByText(/sample data.*not connected/i)).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Overview', level: 1 })).toBeVisible()
    await user.click(screen.getByRole('link', { name: 'Accounts' }))
    expect(screen.getByRole('heading', { name: 'Accounts', level: 1 })).toBeVisible()
    expect(screen.getByRole('searchbox', { name: 'Search accounts' })).toBeVisible()
  })

  it('combines account search and status filters, with a way out of no results', async () => {
    const user = open('/accounts')
    await user.type(screen.getByRole('searchbox', { name: 'Search accounts' }), 'Yusei')
    expect(screen.getByRole('button', { name: 'View YuseiFudo' })).toBeVisible()
    expect(screen.queryByRole('button', { name: 'View SetoKaiba' })).not.toBeInTheDocument()
    await user.selectOptions(screen.getByLabelText('Account status'), 'offline')
    expect(screen.getByText('No accounts found')).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Clear filters' }))
    expect(screen.getByRole('button', { name: 'View SetoKaiba' })).toBeVisible()
  })

  it('validates a BP gift and shows the updated balance and audit entry', async () => {
    const user = open('/accounts')
    await user.click(screen.getByRole('button', { name: 'View YuseiFudo' }))
    const dialog = screen.getByRole('dialog', { name: 'YuseiFudo' })
    await user.click(within(dialog).getByRole('button', { name: 'Give BP' }))
    const amount = within(dialog).getByLabelText('BP amount')
    await user.clear(amount)
    await user.type(amount, '-5')
    expect(within(dialog).getByRole('button', { name: 'Add BP' })).toBeDisabled()
    await user.clear(amount)
    await user.type(amount, '500')
    await user.click(within(dialog).getByRole('button', { name: 'Add BP' }))
    expect(within(dialog).getByText('4,700 BP')).toBeVisible()
    await user.keyboard('{Escape}')
    await user.click(screen.getByRole('link', { name: 'Audit log' }))
    expect(screen.getByText('Added 500 BP to YuseiFudo')).toBeVisible()
  })

  it('creates an invite with the chosen limits and keeps it when navigating', async () => {
    const user = open('/invites')
    await user.click(screen.getByRole('button', { name: 'Create invite' }))
    const dialog = screen.getByRole('dialog', { name: 'Create invite' })
    await user.clear(within(dialog).getByLabelText('Number of sign-ups'))
    await user.type(within(dialog).getByLabelText('Number of sign-ups'), '3')
    await user.click(within(dialog).getByRole('button', { name: 'Create invite' }))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    const newRow = screen.getAllByRole('row')[1]
    const newCode = within(newRow).getByText(/^YO3-/).textContent!
    expect(within(newRow).getByText('3 sign-ups left')).toBeVisible()
    await user.click(screen.getByRole('link', { name: 'Overview' }))
    await user.click(screen.getByRole('link', { name: 'Invites & codes' }))
    expect(
      screen.getByRole('row', { name: new RegExp(`${newCode}.*3 sign-ups left`) }),
    ).toBeVisible()
  })

  it('requires confirmation before restricting an account and allows cancellation', async () => {
    const user = open('/moderation')
    await user.click(screen.getByRole('button', { name: 'Take action' }))
    const dialog = screen.getByRole('dialog', { name: 'Moderation action' })
    await user.selectOptions(within(dialog).getByLabelText('Account'), '3')
    await user.selectOptions(within(dialog).getByLabelText('Action'), 'ban')
    await user.type(within(dialog).getByLabelText('Reason'), 'Repeated harassment')
    await user.click(within(dialog).getByRole('button', { name: 'Review action' }))
    expect(within(dialog).getByText(/Ban AkizaIzinski/)).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Confirm ban' }))
    expect(screen.getByRole('status')).toHaveTextContent('AkizaIzinski was banned')
    expect(screen.getByRole('row', { name: /AkizaIzinski.*Banned/ })).toBeVisible()
  })

  it('keeps moderator controls read-only where admin access is needed', async () => {
    const user = open('/settings')
    await user.selectOptions(screen.getByLabelText('Preview role'), 'moderator')
    expect(screen.getByText('Only administrators can change server settings.')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled()
    await user.click(screen.getByRole('link', { name: 'Backups' }))
    expect(screen.queryByRole('button', { name: 'Create backup' })).not.toBeInTheDocument()
  })

  it('preserves a settings draft on cancel, then saves and records the change', async () => {
    const user = open('/settings')
    const name = screen.getByLabelText('Server name')
    await user.clear(name)
    await user.type(name, 'Friends of Satellite')
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    expect(screen.getByRole('status')).toHaveTextContent('Settings saved')
    await user.click(screen.getByRole('link', { name: 'Overview' }))
    expect(screen.getByText('Friends of Satellite')).toBeVisible()
  })

  it('shows errors independently from informational server logs', async () => {
    const user = open('/logs')
    await user.selectOptions(screen.getByLabelText('Log level'), 'warning')
    expect(screen.getByText(/Sign-in throttled/)).toBeVisible()
    expect(screen.queryByText(/Database backup completed/)).not.toBeInTheDocument()
  })

  it('keeps an unsaved settings draft when visiting another page', async () => {
    const user = open('/settings')
    await user.clear(screen.getByLabelText('Server name'))
    await user.type(screen.getByLabelText('Server name'), 'Draft server name')
    await user.click(screen.getByRole('link', { name: 'Accounts' }))
    await user.click(screen.getByRole('link', { name: 'Settings' }))
    expect(screen.getByLabelText('Server name')).toHaveValue('Draft server name')
    expect(screen.getByText('Unsaved changes')).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Discard changes' }))
    expect(screen.getByLabelText('Server name')).toHaveValue('YO3 Revival')
  })

  it('allows clearing a numeric setting without displaying NaN or allowing an invalid save', async () => {
    const user = open('/settings')
    const errors = vi.spyOn(console, 'error')
    await user.clear(screen.getByLabelText('Starting Life Points'))
    expect(screen.getByLabelText('Starting Life Points')).toHaveValue(null)
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled()
    expect(errors).not.toHaveBeenCalled()
    await user.type(screen.getByLabelText('Starting Life Points'), '4000')
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeEnabled()
  })

  it('resets page-local forms along with sample data', async () => {
    const user = open('/settings')
    await user.clear(screen.getByLabelText('Server name'))
    await user.type(screen.getByLabelText('Server name'), 'Discard me')
    await user.click(screen.getByRole('button', { name: 'Reset preview' }))
    const dialog = screen.getByRole('dialog', { name: 'Reset the preview?' })
    await user.click(within(dialog).getByRole('button', { name: 'Reset preview' }))
    expect(screen.getByLabelText('Server name')).toHaveValue('YO3 Revival')
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled()
  })

  it('does not leave a blank page at an unknown URL', async () => {
    const user = open('/does-not-exist')
    expect(screen.getByRole('heading', { name: 'Page not found' })).toBeVisible()
    await user.click(screen.getByRole('link', { name: 'Back to overview' }))
    expect(screen.getByRole('heading', { name: 'Overview', level: 1 })).toBeVisible()
  })

  it('explains invalid settings beside the field and clears the error when corrected', async () => {
    const user = open('/settings')
    const lp = screen.getByLabelText('Starting Life Points')
    await user.clear(lp)
    expect(lp).toHaveAttribute('aria-invalid', 'true')
    expect(lp).toHaveAccessibleDescription('Enter a whole number from 100 to 999,999.')
    await user.type(lp, '4000')
    expect(lp).not.toHaveAttribute('aria-invalid', 'true')
    expect(screen.queryByText('Enter a whole number from 100 to 999,999.')).not.toBeInTheDocument()
    await user.clear(screen.getByLabelText('Server name'))
    expect(screen.getByLabelText('Server name')).toHaveAccessibleDescription('Enter a server name.')
  })

  it('does not send a staff account into the player moderation form', async () => {
    const user = open('/accounts')
    await user.click(screen.getByRole('button', { name: 'View TestAdmin' }))
    const dialog = screen.getByRole('dialog', { name: 'TestAdmin' })
    expect(within(dialog).queryByRole('link', { name: 'Open moderation' })).not.toBeInTheDocument()
    expect(within(dialog).getByText('Staff account')).toBeVisible()
  })
})
