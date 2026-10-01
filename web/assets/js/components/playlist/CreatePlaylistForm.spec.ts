import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { usePlaylistStore } from '@/stores/playlistStore'
import Component from './CreatePlaylistForm.vue'

describe('createPlaylistForm.vue', () => {
  const h = createHarness()

  const renderComponent = (folder?: PlaylistFolder, playables?: Playable[]) => {
    folder = folder ?? h.factory('playlist-folder').make()
    playables = playables ?? h.factory('song').make(2)

    const rendered = h.render(Component, {
      props: {
        folder,
        playables,
      },
    })

    return {
      ...rendered,
      folder,
      playables,
    }
  }

  it('creates playlist with no playables', async () => {
    const { folder } = renderComponent(undefined, [])
    const storeMock = h.mock(usePlaylistStore(), 'store').mockResolvedValue(h.factory('playlist').make())
    expect(screen.queryByTestId('from-playables')).toBeNull()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'My playlist')
    await h.type(screen.getByRole('textbox', { name: 'Description' }), 'Some description')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    expect(storeMock).toHaveBeenCalledWith(
      {
        name: 'My playlist',
        description: 'Some description',
        folder_id: folder.id,
        folder_name: null,
        cover: null,
      },
      [],
    )
  })

  it('creates playlist with playables', async () => {
    const storeMock = h.mock(usePlaylistStore(), 'store').mockResolvedValue(h.factory('playlist').make())
    const { folder, playables } = renderComponent()

    screen.getByText(`from ${playables.length} songs`)

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'My playlist')
    await h.type(screen.getByRole('textbox', { name: 'Description' }), 'Some description')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    expect(storeMock).toHaveBeenCalledWith(
      {
        name: 'My playlist',
        description: 'Some description',
        folder_id: folder.id,
        folder_name: null,
        cover: null,
      },
      playables,
    )
  })
})
