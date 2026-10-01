import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { ManagedAccount } from '@/services/adminService'
import Component from './PendingRegistrations.vue'

const carol: ManagedAccount = {
  id: 3,
  username: 'carol',
  email: 'carol@example.com',
  email_verified: false,
  role: 'user',
  status: 'pending',
  password_change_required: false,
  created_at: new Date().toISOString(),
  last_seen: null,
  songs: 0,
  bytes: 0,
  exclusive_bytes: 0,
  youtube_music: 'none',
}

describe('pendingRegistrations.vue', () => {
  const h = createHarness()

  it('approves or denies each request', async () => {
    const { emitted } = h.render(Component, { props: { requests: [carol] } })

    screen.getByText('carol')
    screen.getByText(/carol@example\.com/)
    await h.user.click(screen.getByRole('button', { name: 'Approve' }))
    await h.user.click(screen.getByRole('button', { name: 'Deny' }))

    expect(emitted().approve).toEqual([[carol]])
    expect(emitted().deny).toEqual([[carol]])
  })
})
