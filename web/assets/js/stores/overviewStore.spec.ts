import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useOverviewStore } from '@/stores/overviewStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { usePlayableStore } from '@/stores/playableStore'

describe('overviewStore', () => {
  // Each spec has stores of its own, fresh.
  const h = createHarness()

  it('refreshes the store', () => {
    const mostPlayedSongs = h.factory('song').make(6)
    const recentlyPlayedSongs = h.factory('song').make(6)

    const mostPlayedSongsMock = h.mock(usePlayableStore(), 'getMostPlayedSongs', mostPlayedSongs)
    useRecentlyPlayedStore().excerptState.playables = recentlyPlayedSongs

    useOverviewStore().refreshPlayStats()

    expect(mostPlayedSongsMock).toHaveBeenCalled()

    expect(useOverviewStore().state.recentlyPlayed).toEqual(recentlyPlayedSongs)
    expect(useOverviewStore().state.mostPlayedSongs).toEqual(mostPlayedSongs)
  })
})
