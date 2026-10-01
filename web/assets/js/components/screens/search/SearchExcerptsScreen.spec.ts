import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import { useSearchStore } from '@/stores/searchStore'
import Component from './SearchExcerptsScreen.vue'

describe('searchExcerptsScreen.vue', () => {
  const h = createHarness()

  it('executes searching when the search keyword is changed', async () => {
    const mock = h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    h.render(Component)

    eventBus.emit('SEARCH_KEYWORDS_CHANGED', 'search me')

    await waitFor(() => expect(mock).toHaveBeenCalledWith('search me'))
  })

  it('offers to search YouTube Music for the same words', async () => {
    h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    h.render(Component)

    eventBus.emit('SEARCH_KEYWORDS_CHANGED', 'lo & behold')

    const link = await screen.findByRole('link', { name: /Search YouTube Music for “lo & behold”/ })
    expect(link.getAttribute('href')).toMatch(/discover\?q=lo%20%26%20behold$/)
  })
})
