import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useOverviewStore } from '@/stores/overviewStore'
import Router from '@/router'
import Component from './RecentlyPlayedPlayables.vue'

describe('recentlyPlayedPlayables.vue', () => {
  const h = createHarness()

  it('displays the songs', async () => {
    useOverviewStore().state.recentlyPlayed = h.factory('song').make(6)
    h.render(Component)
    await waitFor(() => expect(screen.getAllByTestId('song-card')).toHaveLength(6))
  })

  it('goes to dedicated screen', async () => {
    useOverviewStore().state.recentlyPlayed = h.factory('song').make(6)
    const mock = h.mock(Router, 'go')
    h.render(Component)

    await h.user.click(screen.getByRole('button', { name: 'View all' }))

    expect(mock).toHaveBeenCalledWith('/recently-played')
  })
})
