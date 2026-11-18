import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import { Field } from './ui'
import { Input } from './form-controls'

it('clears a stale constraint error when a parent replaces the controlled value', async () => {
  const user = userEvent.setup()
  const onChange = vi.fn()
  const field = (value: string) => (
    <Field id="amount" label="Amount" hint="100–99999.">
      <Input
        id="amount"
        aria-describedby="amount-hint"
        type="number"
        value={value}
        onChange={onChange}
        required
        min={100}
        max={99999}
      />
    </Field>
  )
  const view = render(field(''))
  const input = screen.getByLabelText('Amount')
  await user.click(input)
  await user.tab()
  expect(input).toHaveAttribute('aria-invalid', 'true')
  expect(input).toHaveAccessibleDescription('100–99999. Enter a value.')
  view.rerender(field('8000'))
  expect(input).not.toHaveAttribute('aria-invalid', 'true')
  expect(screen.queryByText('Enter a value.')).not.toBeInTheDocument()
  expect(input).toHaveAccessibleDescription('100–99999.')
})
