import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { adminService } from '@/services/adminService'
import type { ManagedAccount } from '@/services/adminService'
import { serverSettingsService } from '@/services/serverSettingsService'
import Component from './UsersSettings.vue'

const account = (changes: Partial<ManagedAccount>): ManagedAccount => ({
  id: 2,
  username: 'bob',
  email: null,
  email_verified: false,
  role: 'user',
  status: 'active',
  password_change_required: false,
  created_at: '2026-09-30T00:00:00Z',
  last_seen: null,
  songs: 0,
  bytes: 0,
  exclusive_bytes: 0,
  youtube_music: 'none',
  ...changes,
})

describe('usersSettings.vue', () => {
  const h = createHarness()

  const renderComponent = async () => {
    h.mock(adminService, 'users').mockResolvedValue([
      account({ id: 2, username: 'bob', sso: true }),
      account({ id: 3, username: 'carol', status: 'pending' }),
    ])
    h.mock(adminService, 'storage').mockResolvedValue({ files: 4, bytes: 1024, shared_files: 0 })
    h.mock(serverSettingsService, 'get').mockResolvedValue({ mail_ready: false } as never)
    h.render(Component)
    await screen.findByText('bob')
  }

  it('lists the accounts, with requests apart', async () => {
    await renderComponent()

    screen.getByText('carol')
    screen.getByText('SSO')
    expect(screen.queryByTestId('add-account-form')).toBeNull()
  })

  it('shows the form to add an account on request, and hides it on cancel', async () => {
    await renderComponent()

    await h.user.click(screen.getByRole('button', { name: /Add an account/ }))
    screen.getByTestId('add-account-form')
    expect(screen.queryByRole('button', { name: /Add an account/ })).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByTestId('add-account-form')).toBeNull()
    screen.getByRole('button', { name: /Add an account/ })
  })
})
