import { screen } from '@testing-library/vue'
import { afterEach, describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import Component from './Auth.vue'

describe('auth.vue', () => {
  const h = createHarness({
    authenticated: false,
    beforeEach: () =>
      h
        .mock(authService, 'status')
        .mockResolvedValue({ claimed: true, password_reset: false, registration: false, sso: null }),
  })

  afterEach(() => history.replaceState(null, '', '/'))

  it('renders the credentials form by default', async () => {
    h.render(Component)

    await screen.findByTestId('login-form')
  })

  it('offers a password reset only when email works', async () => {
    h.render(Component)

    await screen.findByTestId('login-form')
    expect(screen.queryByRole('button', { name: 'Forgot password?' })).toBeNull()
  })

  it('emails a reset link to whoever forgot their password', async () => {
    h.mock(authService, 'status').mockResolvedValue({
      claimed: true,
      password_reset: true,
      registration: false,
      sso: null,
    })
    const forgot = h.mock(authService, 'forgot').mockResolvedValue(undefined)
    h.render(Component)

    await h.user.click(await screen.findByRole('button', { name: 'Forgot password?' }))
    await h.type(await screen.findByLabelText('Username or email'), 'bob')
    await h.user.click(screen.getByRole('button', { name: 'Email me a link' }))

    expect(forgot).toHaveBeenCalledWith('bob')
    await screen.findByText(/píxiū emailed it a link/)

    await h.user.click(screen.getByRole('button', { name: 'Back to sign in' }))
    await screen.findByTestId('login-form')
  })

  it('lets people ask for an account while registration is open', async () => {
    h.mock(authService, 'status').mockResolvedValue({
      claimed: true,
      password_reset: true,
      registration: true,
      sso: null,
    })
    const register = h.mock(authService, 'register').mockResolvedValue(undefined)
    h.render(Component)

    await h.user.click(await screen.findByRole('button', { name: 'Ask for an account' }))
    await h.type(await screen.findByLabelText('Username'), 'carol')
    await h.type(screen.getByLabelText('Email'), 'carol@example.com')
    await h.type(screen.getByLabelText('Password'), 'carol secret')
    await h.type(screen.getByLabelText('Password again'), 'carol secret')
    await h.user.click(screen.getByRole('button', { name: 'Ask for an account' }))

    expect(register).toHaveBeenCalledWith({ username: 'carol', email: 'carol@example.com', password: 'carol secret' })
    await screen.findByText('Thanks for asking')
    screen.getByText(/píxiū emails carol@example\.com how it went/)
  })

  it('offers no registration while it is closed', async () => {
    h.render(Component)

    await screen.findByTestId('login-form')
    expect(screen.queryByRole('button', { name: 'Ask for an account' })).toBeNull()
  })

  it('offers single sign-on when it is set up', async () => {
    h.mock(authService, 'status').mockResolvedValue({
      claimed: true,
      password_reset: false,
      registration: false,
      sso: { name: 'Authelia' },
    })
    h.render(Component)

    const button = await screen.findByTestId('sso-button')
    expect(button.textContent).toContain('Sign in with Authelia')
    expect(button.getAttribute('href')).toMatch(/api\/auth\/oidc\/start$/)
  })

  it('says why a single sign-on came back, once', async () => {
    h.mock(authService, 'status').mockResolvedValue({
      claimed: true,
      password_reset: false,
      registration: false,
      sso: { name: 'Authelia' },
    })
    history.replaceState(null, '', '/?sso_error=unlinked')
    h.render(Component)

    await screen.findByText(/This Authelia account is not linked to a píxiū account/)
    expect(location.search).toBe('')
  })
})
