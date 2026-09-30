import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { DialogBoxStub } from '@/__tests__/stubs'
import { accountService } from '@/services/accountService'
import { authService } from '@/services/authService'
import Component from './LinkedAccountsGroup.vue'

const status = (sso: { name: string } | null) => ({ claimed: true, password_reset: false, registration: false, sso })

describe('linkedAccountsGroup.vue', () => {
  const h = createHarness()

  it('offers to link an account at the provider', async () => {
    h.mock(authService, 'status').mockResolvedValue(status({ name: 'Authelia' }))
    h.mock(accountService, 'identities').mockResolvedValue([])
    h.render(Component)

    await screen.findByRole('button', { name: 'Link your Authelia account' })
  })

  it('lists and unlinks linked accounts', async () => {
    h.mock(authService, 'status').mockResolvedValue(status({ name: 'Authelia' }))
    h.mock(accountService, 'identities').mockResolvedValue([
      {
        id: 7,
        provider: 'Authelia',
        email: 'alice@sso.example.com',
        linked_at: '2026-09-30T00:00:00Z',
        last_login_at: null,
      },
    ])
    const unlink = h.mock(accountService, 'unlinkIdentity').mockResolvedValue(undefined)
    h.mock(DialogBoxStub.value, 'confirm', true)
    h.render(Component)

    await screen.findByText(/Authelia · alice@sso\.example\.com/)
    expect(screen.queryByRole('button', { name: 'Link your Authelia account' })).toBeNull()
    await h.user.click(screen.getByRole('button', { name: 'Unlink' }))
    expect(unlink).toHaveBeenCalledWith(7)
  })

  it('stays out of the way without single sign-on', async () => {
    h.mock(authService, 'status').mockResolvedValue(status(null))
    const identities = h.mock(accountService, 'identities').mockResolvedValue([])
    h.render(Component)

    await h.tick(3)
    expect(identities).toHaveBeenCalled()
    expect(screen.queryByTestId('linked-accounts')).toBeNull()
  })
})
