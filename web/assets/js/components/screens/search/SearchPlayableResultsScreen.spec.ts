import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useSearchStore } from '@/stores/searchStore'
import SearchPlayableResultsScreen from './SearchPlayableResultsScreen.vue'

describe('searchPlayableResultsScreen.vue', () => {
  const h = createHarness()

  it('searches for prop query on created', async () => {
    const resetResultMock = h.mock(useSearchStore(), 'resetPlayableResultState')
    const searchMock = h.mock(useSearchStore(), 'playableSearch')

    await h.visit('/search/songs?q=foo')

    h.render(SearchPlayableResultsScreen)

    expect(resetResultMock).toHaveBeenCalled()
    expect(searchMock).toHaveBeenCalledWith('foo')
  })
})
