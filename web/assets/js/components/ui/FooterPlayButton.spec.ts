import { screen, waitFor } from '@testing-library/vue'
import { ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService } from '@/services/QueuePlaybackService'
import { CurrentStreamableKey } from '@/config/symbols'
import { useCommonStore } from '@/stores/commonStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import Router from '@/router'
import Component from './FooterPlayButton.vue'

describe('footerPlayButton.vue', () => {
  const h = createHarness()

  const renderComponent = (currentPlayable: Playable | null = null) => {
    return h.render(Component, {
      global: {
        provide: {
          [<symbol>CurrentStreamableKey]: ref(currentPlayable),
        },
      },
    })
  }

  it('toggles the playback of current item', async () => {
    h.createAudioPlayer()

    const toggleMock = h.mock(playbackService, 'toggle')
    renderComponent(h.factory('song').make())

    await h.user.click(screen.getByRole('button'))

    expect(toggleMock).toHaveBeenCalled()
  })

  it.each<[string, MethodOf<Required<ReturnType<typeof usePlayableStore>>>, Album['id']]>([
    ['/albums/al-7', 'fetchSongsForAlbum', 'al-7'],
    ['/artists/ar-7', 'fetchSongsForArtist', 'ar-7'],
    ['/playlists/pl-7', 'fetchForPlaylist', 'pl-7'],
  ])('initiates playback for %s', async (hash, fetchMethod, id) => {
    h.createAudioPlayer()

    useCommonStore().state.song_count = 10
    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(usePlayableStore(), fetchMethod).mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    await h.visit(hash)
    renderComponent()

    await h.user.click(screen.getByRole('button'))
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(id)
      expect(playMock).toHaveBeenCalledWith(songs)
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })

  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- vitest it.each typing limitation
  it.each<[string, any, string]>([
    ['/favorites', usePlayableStore, 'fetchFavorites'],
    ['/recently-played', useRecentlyPlayedStore, 'fetch'],
  ])('initiates playback for %s', async (hash, useStore, fetchMethod) => {
    h.createAudioPlayer()

    useCommonStore().state.song_count = 10
    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(useStore(), fetchMethod).mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    await h.visit(hash)
    renderComponent()

    await h.user.click(screen.getByRole('button'))
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled()
      expect(playMock).toHaveBeenCalledWith(songs)
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })

  it('does nothing if there are no songs', async () => {
    h.createAudioPlayer()

    useCommonStore().state.song_count = 0

    const playMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    await h.visit('songs')
    renderComponent()

    await h.user.click(screen.getByRole('button'))
    await waitFor(() => {
      expect(playMock).not.toHaveBeenCalled()
      expect(goMock).not.toHaveBeenCalled()
    })
  })
})
