import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useCommonStore } from '@/stores/commonStore'
import { useGenreStore } from '@/stores/genreStore'
import Component from './GenreListScreen.vue'

describe('genreListScreen', () => {
  const h = createHarness()

  const renderComponent = async (genres?: Genre[]) => {
    genres = genres || h.factory('genre').make(5)
    const fetchMock = h.mock(useGenreStore(), 'fetchAll').mockResolvedValue(genres)

    const rendered = h.render(Component, {
      global: {
        stubs: {
          GenreCard: h.stub('genre-card'),
        },
      },
    })

    return {
      genres,
      fetchMock,
      ...rendered,
    }
  }

  it('renders the list of genres', async () => {
    await renderComponent()
    await waitFor(() => expect(screen.queryAllByTestId('genre-card')).toHaveLength(5))
  })

  it('shows a message when the library is empty', async () => {
    useCommonStore().state.song_length = 0
    const { fetchMock } = await renderComponent()

    await waitFor(() => {
      expect(fetchMock).not.toHaveBeenCalled()
      screen.getByTestId('screen-empty-state')
    })
  })

  it('says so when the library has no genres yet', async () => {
    useCommonStore().state.song_length = 10
    await renderComponent([])

    await screen.findByText('No genres yet.')
  })
})
