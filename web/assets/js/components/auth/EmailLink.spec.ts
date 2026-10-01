import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { HTTPError } from 'ky'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import { logger } from '@/utils/logger'
import Component from './EmailLink.vue'

const expired = () => {
  const data = { message: 'This link has expired or was already used.', code: 'expired' }
  const error = new HTTPError(
    new Response(JSON.stringify(data), { status: 410 }),
    new Request('http://test/api/auth/reset'),
    {} as any,
  )
  error.data = data
  return error
}

describe('emailLink.vue', () => {
  const h = createHarness()

  it('confirms an email address', async () => {
    const verify = h.mock(authService, 'verifyEmail').mockResolvedValue(false)
    h.visit('/verify-email/abc_DEF-123')
    h.render(Component)

    await screen.findByText('Your email is confirmed')
    expect(verify).toHaveBeenCalledWith('abc_DEF-123')
    screen.getByRole('button', { name: 'Open píxiū' })
  })

  it('says when confirming opened an approved account', async () => {
    h.mock(authService, 'verifyEmail').mockResolvedValue(true)
    h.visit('/verify-email/abc')
    h.render(Component)

    await screen.findByText('Your account is ready')
  })

  it('says when a confirmation link no longer works, which is no error', async () => {
    const error = vi.spyOn(logger, 'error')
    h.mock(authService, 'verifyEmail').mockRejectedValue(expired())
    h.visit('/verify-email/used')
    h.render(Component)

    await screen.findByText('This link no longer works')
    expect(error).not.toHaveBeenCalled()
  })

  it('logs a confirmation that failed otherwise', async () => {
    const error = vi.spyOn(logger, 'error').mockImplementation(() => {})
    h.mock(authService, 'verifyEmail').mockRejectedValue(new Error('offline'))
    h.visit('/verify-email/abc')
    h.render(Component)

    await screen.findByText('That did not work')
    expect(error).toHaveBeenCalled()
  })

  it('sets a new password from a reset link', async () => {
    const reset = h.mock(authService, 'resetPassword').mockResolvedValue(undefined)
    h.visit('/reset-password/tok3n')
    h.render(Component)

    await h.type(screen.getByLabelText('New password'), 'a fresh one')
    await h.type(screen.getByLabelText('New password again'), 'a fresh one')
    expect(screen.queryByText('The passwords differ.')).toBeNull()
    await h.user.click(screen.getByRole('button', { name: 'Set password' }))

    expect(reset).toHaveBeenCalledWith('tok3n', 'a fresh one')
  })

  it('sends people back when a reset link no longer works', async () => {
    h.mock(authService, 'resetPassword').mockRejectedValue(expired())
    h.visit('/reset-password/used')
    h.render(Component)

    await h.type(screen.getByLabelText('New password'), 'a fresh one')
    await h.type(screen.getByLabelText('New password again'), 'a fresh one')
    await h.user.click(screen.getByRole('button', { name: 'Set password' }))

    await screen.findByText(/This link expired, or was used already/)
    screen.getByRole('button', { name: 'Back to sign in' })
    expect(screen.queryByLabelText('New password')).toBeNull()
  })
})
