import { waitFor } from '@testing-library/vue'
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
})
