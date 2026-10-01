import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useGenreStore } from '@/stores/genreStore'
import { usePlayableStore } from '@/stores/playableStore'
import Component from './GenreScreen.vue'

describe('genreScreen', () => {
  const h = createHarness()

  const renderComponent = async (genre?: Genre, songs?: Song[]) => {
    genre = genre || h.factory('genre').make()

    const fetchGenreMock = h.mock(useGenreStore(), 'fetchOne').mockResolvedValue(genre)
    const paginateMock = h.mock(usePlayableStore(), 'paginateSongsByGenre').mockResolvedValue({
      nextCursor: 'next-token',
      items: songs || h.factory('song').make(13),
    })

    const rendered = (await h.visit(`genres/${genre.id}`)).render(Component, {
      global: {
        stubs: {
          SongList: h.stub('song-list'),
        },
      },
    })

    await waitFor(() => {
      expect(fetchGenreMock).toHaveBeenCalledWith(genre!.id)
      expect(paginateMock).toHaveBeenCalledWith(genre!.id, {
        sort: 'title',
        order: 'asc',
        cursor: '',
      })
    })

    await h.tick(2)

    return {
      ...rendered,
      genre,
    }
  }

  it('renders the song list', async () => {
    await renderComponent()
    screen.getByTestId('song-list')
  })
})
