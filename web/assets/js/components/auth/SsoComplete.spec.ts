import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { HTTPError } from 'ky'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import { logger } from '@/utils/logger'
import Component from './SsoComplete.vue'

const expired = () => {
  const data = { message: 'This sign-in expired; sign in again.', code: 'expired' }
  const error = new HTTPError(
    new Response(JSON.stringify(data), { status: 410 }),
    new Request('http://test/api/auth/oidc/exchange'),
    {} as any,
  )
  error.data = data
  return error
}

describe('ssoComplete.vue', () => {
  const h = createHarness()

  it('says when the sign-in code no longer works, which is no error', async () => {
    const error = vi.spyOn(logger, 'error')
    const exchange = h.mock(authService, 'exchangeSsoCode').mockRejectedValue(expired())
    h.visit('/sso/abc_DEF-1')
    h.render(Component)

    await screen.findByText('That sign-in expired')
    expect(exchange).toHaveBeenCalledWith('abc_DEF-1')
    screen.getByRole('button', { name: 'Back to sign in' })
    expect(error).not.toHaveBeenCalled()
  })

  it('logs an exchange that failed otherwise', async () => {
    const error = vi.spyOn(logger, 'error').mockImplementation(() => {})
    h.mock(authService, 'exchangeSsoCode').mockRejectedValue(new Error('offline'))
    h.visit('/sso/abc')
    h.render(Component)

    await screen.findByText('That sign-in expired')
    expect(error).toHaveBeenCalled()
  })
})
