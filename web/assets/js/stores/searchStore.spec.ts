import { describe, expect, it } from 'vite-plus/test'
import { reactive } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { ExcerptState } from '@/stores/searchStore'
import { searchStore } from '@/stores/searchStore'

describe('searchStore', () => {
  const h = createHarness({
    beforeEach: () => {
      searchStore.state = reactive<{
        excerpt: ExcerptState
        playables: Playable[]
      }>({
        excerpt: {
          playables: [],
          albums: [],
          artists: [],
          podcasts: [],
          radio_stations: [],
        },
        playables: [],
      })
    },
  })

  it('resets the song result state', () => {
    searchStore.state.playables = h.factory('song').make(3)
    searchStore.resetPlayableResultState()
    expect(searchStore.state.playables).toEqual([])
  })
})
