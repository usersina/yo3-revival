import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { vi, expect, test, afterEach } from 'vitest'
import { CodeStep, QrCode, RecoveryCodes } from './ConnectedMfa'

afterEach(() => vi.unstubAllGlobals())

test('the QR code is an accessible inline SVG of the otpauth link, dark on white', () => {
  const { container } = render(
    <QrCode value="otpauth://totp/YO3%20Revival:Yugi?secret=GEZDGNBVGY3TQOJQ&issuer=YO3%20Revival" />,
  )
  const svg = screen.getByRole('img', { name: 'QR code for your authenticator app' })
  expect(svg.tagName.toLowerCase()).toBe('svg')
  expect(container.querySelector('rect')?.getAttribute('fill')).toBe('#ffffff')
  expect(container.querySelector('path')?.getAttribute('d')?.length).toBeGreaterThan(100)
})

test('recovery codes can be continued from only after the owner confirms they are saved', async () => {
  const done = vi.fn()
  render(
    <RecoveryCodes
      codes={['ABCD-EFGH-JKLM-NPQR', 'STUV-WXYZ-2345-6789']}
      doneLabel="Continue"
      onDone={done}
    />,
  )
  expect(screen.getByRole('list', { name: 'Recovery codes' }).children).toHaveLength(2)
  const button = screen.getByRole('button', { name: 'Continue' })
  expect(button).toBeDisabled()
  fireEvent.click(screen.getByLabelText('I saved these recovery codes'))
  expect(button).toBeEnabled()
  fireEvent.click(button)
  expect(done).toHaveBeenCalledOnce()
})

test('the code step sends the challenge with the code, and offers a recovery code and a way back', async () => {
  const fetcher = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ code: 'invalid_code', message: 'That code is not right.' }), {
      status: 403,
      headers: { 'Content-Type': 'application/json' },
    }),
  )
  vi.stubGlobal('fetch', fetcher)
  const back = vi.fn()
  const challenge = 'a'.repeat(64)
  render(
    <QueryClientProvider client={new QueryClient()}>
      <CodeStep challenge={challenge} onBack={back} />
    </QueryClientProvider>,
  )
  const user = userEvent.setup()
  await user.type(screen.getByLabelText('Authenticator code'), '123456')
  await user.click(screen.getByRole('button', { name: 'Verify and sign in' }))
  await waitFor(() =>
    expect(screen.getByRole('alert')).toHaveTextContent('That code is not right.'),
  )
  const [url, init] = fetcher.mock.calls[0]
  expect(url).toBe('/panel/api/v1/sign-in/code')
  expect(JSON.parse(init.body)).toEqual({ challenge, code: '123456' })
  await user.click(screen.getByRole('button', { name: 'Use a recovery code' }))
  expect(screen.getByLabelText('Recovery code')).toHaveValue('')
  await user.click(screen.getByRole('button', { name: 'Back to password' }))
  expect(back).toHaveBeenCalledOnce()
})
