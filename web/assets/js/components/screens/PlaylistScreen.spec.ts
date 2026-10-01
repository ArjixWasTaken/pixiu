import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import { usePlaylistStore } from '@/stores/playlistStore'
import { usePlayableStore } from '@/stores/playableStore'
import type { Events } from '@/config/events'
import Component from './PlaylistScreen.vue'

vi.mock('@/composables/useContextMenu')

describe('playlistScreen.vue', () => {
  const h = createHarness()

  const renderComponent = async (songs: Playable[] = []) => {
    const playlist = h.factory('playlist').make()
    h.actingAsUser(h.factory('user').state('current').make({ id: playlist.owner_id }) as CurrentUser)

    usePlaylistStore().state.playlists = []
    usePlaylistStore().init([playlist])
    playlist.playables = songs

    const fetchSongsMock = h.mock(usePlayableStore(), 'fetchForPlaylist').mockResolvedValueOnce(songs)

    const rendered = h.render(Component, {
      global: {
        stubs: {
          FavoriteButton: h.stub('favorite-button', true),
        },
      },
    })

    await h.visit(`playlists/${playlist.id}`)

    await waitFor(() => expect(fetchSongsMock).toHaveBeenCalledWith(playlist, false))

    return {
      ...rendered,
      playlist,
      fetchSongsMock,
    }
  }

  it.each<[keyof Events]>([['PLAYLIST_UPDATED']])('refreshes upon %s event trigger', async eventKey => {
    const { playlist, fetchSongsMock } = await renderComponent()
    fetchSongsMock.mockResolvedValueOnce(h.factory('song').make(5))

    eventBus.emit(eventKey, playlist)

    expect(fetchSongsMock).toHaveBeenCalledWith(playlist, false)
  })

  it('shows the playlist opened last, though the one before it loads after', async () => {
    const [first, second] = h.factory('playlist').make({ is_smart: false }, 2)
    h.actingAsUser(h.factory('user').state('current').make({ id: first.owner_id }) as CurrentUser)
    usePlaylistStore().state.playlists = []
    usePlaylistStore().init([first, second])
    // As fetching them would.
    second.playables = []

    let firstLoaded!: (songs: Playable[]) => void
    const fetchSongsMock = h
      .mock(usePlayableStore(), 'fetchForPlaylist')
      .mockImplementationOnce(() => new Promise(resolve => (firstLoaded = resolve)))
      .mockResolvedValueOnce([])

    h.render(Component)
    await h.visit(`playlists/${first.id}`)
    await waitFor(() => expect(fetchSongsMock).toHaveBeenCalledTimes(1))

    await h.visit(`playlists/${second.id}`)
    await screen.findByText('The playlist is empty.')

    firstLoaded(h.factory('song').make(3))
    await h.tick(2)

    // Still the second playlist, empty.
    screen.getByText('The playlist is empty.')
  })
})
