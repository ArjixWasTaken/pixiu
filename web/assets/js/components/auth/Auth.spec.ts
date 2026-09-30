import { screen, waitFor } from '@testing-library/vue'
import { afterEach, describe, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import Component from './Auth.vue'

describe('auth.vue', () => {
  const h = createHarness({
    authenticated: false,
    beforeEach: () => h.mock(authService, 'claimed').mockResolvedValue(true),
  })

  afterEach(() => (window.KOEL.sso_providers = []))

  it('renders the credentials form by default', async () => {
    h.render(Component)

    await screen.findByTestId('login-form')
  })

  it('shows the SSO login options', async () => {
    window.KOEL.sso_providers = ['Google']

    h.render(Component, {
      global: {
        stubs: {
          GoogleLoginButton: h.stub('google-login-button'),
        },
      },
    })

    await waitFor(() => screen.getByTestId('google-login-button'))
  })
})
