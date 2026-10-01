import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { searchStore } from '@/stores/searchStore'
import SearchPlayableResultsScreen from './SearchPlayableResultsScreen.vue'

describe('searchPlayableResultsScreen.vue', () => {
  const h = createHarness()

  it('searches for prop query on created', async () => {
    const resetResultMock = h.mock(searchStore, 'resetPlayableResultState')
    const searchMock = h.mock(searchStore, 'playableSearch')

    await h.visit('/search/songs?q=foo')

    h.render(SearchPlayableResultsScreen)

    expect(resetResultMock).toHaveBeenCalled()
    expect(searchMock).toHaveBeenCalledWith('foo')
  })
})
