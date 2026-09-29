import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { overviewStore } from '@/stores/overviewStore'
import { recentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { playableStore } from '@/stores/playableStore'

describe('overviewStore', () => {
  const h = createHarness({
    beforeEach: () => {
      overviewStore.state = {
        recentlyPlayed: [],
        recentlyAddedSongs: [],
        recentlyAddedAlbums: [],
        recentlyAddedArtists: [],
        mostPlayedSongs: [],
        mostPlayedAlbums: [],
        mostPlayedArtists: [],
        leastPlayedSongs: [],
        randomAlbums: [],
        randomArtists: [],
        randomSongs: [],
        similarSongs: [],
      }
    },
  })

  it('refreshes the store', () => {
    const mostPlayedSongs = h.factory('song').make(6)
    const recentlyPlayedSongs = h.factory('song').make(6)

    const mostPlayedSongsMock = h.mock(playableStore, 'getMostPlayedSongs', mostPlayedSongs)
    recentlyPlayedStore.excerptState.playables = recentlyPlayedSongs

    overviewStore.refreshPlayStats()

    expect(mostPlayedSongsMock).toHaveBeenCalled()

    expect(overviewStore.state.recentlyPlayed).toEqual(recentlyPlayedSongs)
    expect(overviewStore.state.mostPlayedSongs).toEqual(mostPlayedSongs)
  })
})
