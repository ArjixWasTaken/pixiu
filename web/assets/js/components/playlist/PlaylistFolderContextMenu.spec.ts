import { describe, expect, it, vi } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { assertOpenModal } from '@/__tests__/assertions'
import { DialogBoxStub, MessageToasterStub } from '@/__tests__/stubs'
import { usePlayableStore } from '@/stores/playableStore'
import { playbackService } from '@/services/QueuePlaybackService'
import Router from '@/router'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
import EditPlaylistFolderForm from '@/components/playlist/EditPlaylistFolderForm.vue'
import CreatePlaylistFolderForm from '@/components/playlist/CreatePlaylistFolderForm.vue'

const openModalMock = vi.fn()

vi.mock('@/composables/useModal', () => ({
  useModal: () => ({
    openModal: openModalMock,
  }),
}))

import Component from './PlaylistFolderContextMenu.vue'

describe('playlistFolderContextMenu.vue', () => {
  const h = createHarness({
    beforeEach: () => openModalMock.mockClear(),
  })

  const renderComponent = async (folder?: PlaylistFolder) => {
    folder = folder || h.factory('playlist-folder').make()

    const rendered = h.renderMenu(Component, {
      props: {
        folder,
      },
    })

    return {
      ...rendered,
      folder,
    }
  }

  const createPlayableFolder = () => {
    const folder = h.factory('playlist-folder').make()
    const playlists = h.factory('playlist').make({ folder_id: folder.id }, 3)
    h.mock(usePlaylistFolderStore(), 'playlistsInTree', playlists)

    return { folder, playlists }
  }

  it('edits', async () => {
    const { folder } = await renderComponent()

    await h.user.click(screen.getByText('Edit…'))

    await assertOpenModal(openModalMock, EditPlaylistFolderForm, { folder })
  })

  it('deletes', async () => {
    const confirmMock = h.mock(DialogBoxStub.value, 'confirm', true)
    const deleteMock = h.mock(usePlaylistFolderStore(), 'delete')
    const { folder } = await renderComponent()

    await h.user.click(screen.getByText('Delete'))

    expect(confirmMock).toHaveBeenCalledWith(
      `Delete the playlist folder "${folder.name}"? Its playlists and subfolders will be kept.`,
    )
    expect(deleteMock).toHaveBeenCalledWith(folder)
  })

  it('creates a child folder', async () => {
    const { folder } = await renderComponent()

    await h.user.click(screen.getByText('Add'))
    await h.user.click(screen.getByText('New folder…'))

    await assertOpenModal(openModalMock, CreatePlaylistFolderForm, { parent: folder })
  })

  it('plays', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylists').mockResolvedValue(songs)
    const queueMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')
    const { folder, playlists } = createPlayableFolder()
    await renderComponent(folder)

    await h.user.click(screen.getByText('Play all'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlists)
      expect(queueMock).toHaveBeenCalledWith(songs)
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })

  it('warns if attempting to play with no songs in folder', async () => {
    h.createAudioPlayer()

    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylists').mockResolvedValue([])
    const queueMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')
    const warnMock = h.mock(MessageToasterStub.value, 'warning')

    const { folder, playlists } = createPlayableFolder()
    await renderComponent(folder)

    await h.user.click(screen.getByText('Play all'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlists)
      expect(queueMock).not.toHaveBeenCalled()
      expect(goMock).not.toHaveBeenCalled()
      expect(warnMock).toHaveBeenCalledWith('No songs available.')
    })
  })

  it('shuffles', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylists').mockResolvedValue(songs)
    const queueMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')

    const { folder, playlists } = createPlayableFolder()
    await renderComponent(folder)

    await h.user.click(screen.getByText('Shuffle all'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlists)
      expect(queueMock).toHaveBeenCalledWith(songs, true)
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })

  it('does not show shuffle option if folder is empty', async () => {
    await renderComponent()

    expect(screen.queryByText('Shuffle all')).toBeNull()
    expect(screen.queryByText('Play all')).toBeNull()
  })

  it('warns if attempting to shuffle with no songs in folder', async () => {
    h.createAudioPlayer()

    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylists').mockResolvedValue([])
    const queueMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')
    const warnMock = h.mock(MessageToasterStub.value, 'warning')

    const { folder, playlists } = createPlayableFolder()
    await renderComponent(folder)

    await h.user.click(screen.getByText('Shuffle all'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlists)
      expect(queueMock).not.toHaveBeenCalled()
      expect(goMock).not.toHaveBeenCalled()
      expect(warnMock).toHaveBeenCalledWith('No songs available.')
    })
  })
})
