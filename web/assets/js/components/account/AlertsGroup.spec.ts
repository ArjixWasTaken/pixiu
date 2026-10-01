import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { accountService } from '@/services/accountService'
import Component from './AlertsGroup.vue'

describe('alertsGroup.vue', () => {
  const h = createHarness()

  it('switches alerts on and off', async () => {
    h.mock(accountService, 'alerts').mockResolvedValue({
      deliverable: true,
      alerts: { youtube_music_expired: true, watch_failing: true },
    })
    const set = h.mock(accountService, 'setAlerts').mockResolvedValue({
      deliverable: true,
      alerts: { youtube_music_expired: true, watch_failing: false },
    })
    h.render(Component)

    await screen.findByText('A watch keeps failing')
    await h.user.click(screen.getByText('A watch keeps failing'))

    expect(set).toHaveBeenCalledWith({ watch_failing: false })
  })

  it('says when alerts cannot reach the user', async () => {
    h.mock(accountService, 'alerts').mockResolvedValue({
      deliverable: false,
      alerts: { youtube_music_expired: true, watch_failing: true },
    })
    h.render(Component)

    await screen.findByText(/Alerts go to a confirmed email address/)
  })
})
