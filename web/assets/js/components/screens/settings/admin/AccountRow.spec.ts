import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { ManagedAccount } from '@/services/adminService'
import Component from './AccountRow.vue'

const account = (changes: Partial<ManagedAccount> = {}): ManagedAccount => ({
  id: 2,
  username: 'bob',
  email: 'bob@example.com',
  email_verified: true,
  role: 'user',
  status: 'active',
  password_change_required: false,
  created_at: '2026-09-30T00:00:00Z',
  last_seen: null,
  songs: 12,
  bytes: 3 * 1024 * 1024,
  exclusive_bytes: 1024 * 1024,
  youtube_music: 'valid',
  ...changes,
})

describe('accountRow.vue', () => {
  const h = createHarness()

  /** Opens the account's menu, which closes again after each choice. */
  const menu = () => h.user.click(screen.getByTitle('Manage this account'))

  it('shows the account and what its library takes up', () => {
    h.render(Component, { props: { account: account({ role: 'admin' }), isYou: false, mailReady: false } })

    screen.getByText('bob')
    screen.getByText('Admin')
    screen.getByText(/bob@example\.com · 12 songs · 3 MB · never signed in · YouTube Music connected/)
  })

  it('says why an account is not on', () => {
    h.render(Component, { props: { account: account({ status: 'disabled' }), isYou: false, mailReady: false } })
    screen.getByText('Off')
  })

  it('offers its actions as events', async () => {
    const { emitted } = h.render(Component, { props: { account: account(), isYou: false, mailReady: false } })

    await menu()
    await h.user.click(screen.getByText('Make admin'))
    await menu()
    await h.user.click(screen.getByText('Turn off'))
    await menu()
    await h.user.click(screen.getByText('Delete account…'))
    expect(emitted().toggleRole).toHaveLength(1)
    expect(emitted().toggleStatus).toHaveLength(1)
    expect(emitted().remove).toHaveLength(1)

    await menu()
    await h.user.click(screen.getByText('Set a temporary password'))
    await h.type(screen.getByLabelText('Temporary password'), 'temporary!')
    await h.user.click(screen.getByRole('button', { name: 'Set' }))
    expect(emitted().setPassword).toEqual([['temporary!']])
  })

  it('offers a reset link only when email can reach the account', async () => {
    const { emitted, rerender } = h.render(Component, {
      props: { account: account(), isYou: false, mailReady: false },
    })
    await menu()
    expect(screen.queryByText('Email a password reset link')).toBeNull()

    await rerender({ account: account(), isYou: false, mailReady: true })
    await h.user.click(screen.getByText('Email a password reset link'))
    expect(emitted().sendReset).toHaveLength(1)

    await rerender({ account: account({ email: null }), isYou: false, mailReady: true })
    await menu()
    expect(screen.queryByText('Email a password reset link')).toBeNull()
  })

  it('sends an unconfirmed address its link again', async () => {
    const { emitted } = h.render(Component, {
      props: { account: account({ email_verified: false }), isYou: false, mailReady: true },
    })

    await menu()
    await h.user.click(screen.getByText('Send the confirmation link again'))
    expect(emitted().resendVerification).toHaveLength(1)
  })

  it('does not offer to turn off or delete your own account', async () => {
    h.render(Component, { props: { account: account(), isYou: true, mailReady: false } })

    screen.getByText('You')
    await menu()
    screen.getByText('Make admin')
    expect(screen.queryByText('Turn off')).toBeNull()
    expect(screen.queryByText('Delete account…')).toBeNull()
  })
})
