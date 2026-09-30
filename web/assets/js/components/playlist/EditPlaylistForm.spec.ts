import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playlistFolderStore } from '@/stores/playlistFolderStore'
import { playlistStore } from '@/stores/playlistStore'
import { screen, waitFor } from '@testing-library/vue'
import Component from './EditPlaylistForm.vue'

describe('editPlaylistForm.vue', () => {
  const h = createHarness()

  const renderComponent = (playlist?: Playlist) => {
    playlist = playlist || h.factory('playlist').make()
    playlistStore.state.playlists = [playlist]

    const rendered = h.render(Component, {
      props: {
        playlist,
      },
    })

    return {
      ...rendered,
      playlist,
    }
  }

  it('edits the playlist with no changes to cover', async () => {
    const updateMock = h.mock(playlistStore, 'update')
    playlistFolderStore.state.folders = h.factory('playlist-folder').make(3)

    const { playlist } = renderComponent(
      h.factory('playlist').make({
        name: 'My playlist',
        folder_id: playlistFolderStore.state.folders[0].id,
      }),
    )

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Your playlist')
    await h.type(screen.getByRole('textbox', { name: 'Description' }), 'Updated description')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(updateMock).toHaveBeenCalledWith(playlist, {
        name: 'Your playlist',
        description: 'Updated description',
        folder_id: playlist.folder_id,
        folder_name: null,
      })
    })
  })
})
