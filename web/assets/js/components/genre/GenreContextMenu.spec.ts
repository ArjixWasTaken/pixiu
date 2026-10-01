import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService } from '@/services/QueuePlaybackService'
import { usePlayableStore } from '@/stores/playableStore'
import { useQueueStore } from '@/stores/queueStore'
import Router from '@/router'
import Component from './GenreContextMenu.vue'

describe('genreContextMenu.vue', () => {
  const h = createHarness()

  const renderComponent = async (genre?: Genre) => {
    genre =
      genre ||
      h.factory('genre').make({
        name: 'Classical',
      })

    const rendered = h.renderMenu(Component, {
      props: {
        genre,
      },
    })

    return {
      ...rendered,
      genre,
    }
  }

  it('renders', async () => {
    await renderComponent()
    expect(screen.getByRole('menu').innerHTML).toMatchSnapshot()
  })

  it('plays all', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsByGenre').mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    const { genre } = await renderComponent()
    await h.user.click(screen.getByText('Play'))
    await h.tick()

    expect(fetchMock).toHaveBeenCalledWith(genre)
    expect(playMock).toHaveBeenCalledWith(songs)
    expect(goMock).toHaveBeenCalledWith('queue')
  })

  it('shuffles all', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsByGenre').mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    const { genre } = await renderComponent()
    await h.user.click(screen.getByText('Shuffle'))
    await h.tick()

    expect(fetchMock).toHaveBeenCalledWith(genre, true)
    expect(playMock).toHaveBeenCalledWith(songs)
    expect(goMock).toHaveBeenCalledWith('queue')
  })

  it('adds to queue', async () => {
    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsByGenre').mockResolvedValue(songs)
    const queueMock = h.mock(useQueueStore(), 'queue')

    const { genre } = await renderComponent()
    await h.user.click(screen.getByText('Add to queue'))
    await h.tick()

    expect(fetchMock).toHaveBeenCalledWith(genre)
    expect(queueMock).toHaveBeenCalledWith(songs)
  })
})
