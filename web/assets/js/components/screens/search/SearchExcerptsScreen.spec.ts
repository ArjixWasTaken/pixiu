import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useSearchStore } from '@/stores/searchStore'
import { useDiscoverPlatform } from '@/composables/useDiscoverPlatform'
import Component from './SearchExcerptsScreen.vue'

describe('searchExcerptsScreen.vue', () => {
  const h = createHarness()

  it('searches for the words in its URL', async () => {
    const mock = h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    await h.visit('/search?q=search%20me')
    h.render(Component)

    await waitFor(() => expect(mock).toHaveBeenCalledWith('search me'))
    screen.getByText('Results for “search me”')
  })

  it('follows the words as they change', async () => {
    const mock = h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    await h.visit('/search?q=one')
    h.render(Component)

    await h.visit('/search?q=two')

    await waitFor(() => expect(mock).toHaveBeenCalledWith('two'))
  })

  it('offers to search YouTube Music for the same words', async () => {
    h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    await h.visit('/search?q=lo%20%26%20behold')
    h.render(Component)

    const link = await screen.findByRole('link', { name: /Search YouTube Music for “lo & behold”/ })
    expect(link.getAttribute('href')).toMatch(/discover\?q=lo%20%26%20behold$/)
  })

  it('names the platform Discover searches', async () => {
    h.mock(useSearchStore(), 'excerptSearch').mockResolvedValue({ playables: [], albums: [], artists: [] })
    useDiscoverPlatform().platform.value = 'deezer'
    await h.visit('/search?q=dawn')
    h.render(Component)

    await screen.findByRole('link', { name: /Search Deezer for “dawn”/ })
  })
})
