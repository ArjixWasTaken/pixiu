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

  it('shows the account and what its library takes up', () => {
    h.render(Component, { props: { account: account({ role: 'admin' }), isYou: false } })

    screen.getByText('bob')
    screen.getByText('Admin')
    screen.getByText(/bob@example\.com · 12 songs · 3 MB · never signed in · YouTube Music connected/)
  })

  it('says why an account is not on', () => {
    h.render(Component, { props: { account: account({ status: 'disabled' }), isYou: false } })
    screen.getByText('Off')
  })

  it('offers its actions as events', async () => {
    const { emitted } = h.render(Component, { props: { account: account(), isYou: false } })

    await h.user.click(screen.getByText('Make admin'))
    await h.user.click(screen.getByText('Turn off'))
    await h.user.click(screen.getByText('Delete account…'))
    expect(emitted().toggleRole).toHaveLength(1)
    expect(emitted().toggleStatus).toHaveLength(1)
    expect(emitted().remove).toHaveLength(1)

    await h.user.click(screen.getByText('Set a temporary password'))
    await h.type(screen.getByLabelText('Temporary password'), 'temporary!')
    await h.user.click(screen.getByRole('button', { name: 'Set' }))
    expect(emitted().setPassword).toEqual([['temporary!']])
  })

  it('does not offer to turn off or delete your own account', () => {
    h.render(Component, { props: { account: account(), isYou: true } })

    screen.getByText('You')
    expect(screen.queryByText('Turn off')).toBeNull()
    expect(screen.queryByText('Delete account…')).toBeNull()
  })
})
