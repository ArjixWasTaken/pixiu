import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import Component from './SsoComplete.vue'

describe('ssoComplete.vue', () => {
  const h = createHarness()

  it('says when the sign-in code no longer works', async () => {
    const exchange = h.mock(authService, 'exchangeSsoCode').mockRejectedValue(new Error('gone'))
    h.visit('/sso/abc_DEF-1')
    h.render(Component)

    await screen.findByText('That sign-in expired')
    expect(exchange).toHaveBeenCalledWith('abc_DEF-1')
    screen.getByRole('button', { name: 'Back to sign in' })
  })
})
