import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useCommonStore } from '@/stores/commonStore'
import { useGenreStore } from '@/stores/genreStore'
import { logger } from '@/utils/logger'
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

  it('says when the genres couldn’t load, and tries again', async () => {
    useCommonStore().state.song_length = 10
    h.mock(logger, 'error')
    const genres = h.factory('genre').make(2)
    const fetchMock = h.mock(useGenreStore(), 'fetchAll').mockRejectedValueOnce(new Error('offline'))
    fetchMock.mockResolvedValueOnce(genres)
    h.render(Component, { global: { stubs: { GenreCard: h.stub('genre-card') } } })

    await screen.findByText(/Couldn’t load the genres\./)
    expect(screen.queryByText('No genres yet.')).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Try again' }))

    await waitFor(() => expect(screen.queryAllByTestId('genre-card')).toHaveLength(2))
  })
})
