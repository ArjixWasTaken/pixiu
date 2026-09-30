import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { accountService } from '@/services/accountService'
import type { Account } from '@/services/accountService'
import Component from './ProfileGroup.vue'

const me = (changes: Partial<Account> = {}): Account => ({
  id: 1,
  username: 'alice',
  email: 'alice@example.com',
  email_verified: false,
  role: 'admin',
  status: 'active',
  password_change_required: false,
  created_at: '2026-09-30T00:00:00Z',
  mail_ready: true,
  ...changes,
})

describe('profileGroup.vue', () => {
  const h = createHarness()

  it('sends the confirmation link again', async () => {
    h.mock(accountService, 'me').mockResolvedValue(me())
    const resend = h.mock(accountService, 'resendVerification').mockResolvedValue(undefined)
    h.render(Component)

    await screen.findByTestId('email-unconfirmed')
    await h.user.click(screen.getByRole('button', { name: 'Send the link again' }))

    expect(resend).toHaveBeenCalled()
  })

  it('offers no link when email cannot go out, or the address is confirmed', async () => {
    h.mock(accountService, 'me').mockResolvedValue(me({ mail_ready: false }))
    h.render(Component)
    await screen.findByLabelText('Email')
    expect(screen.queryByTestId('email-unconfirmed')).toBeNull()
  })
})
