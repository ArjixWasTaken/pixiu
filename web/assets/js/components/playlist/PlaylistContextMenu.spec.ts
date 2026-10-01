import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { assertOpenModal } from '@/__tests__/assertions'
import factory from '@/__tests__/factory'
import { MessageToasterStub } from '@/__tests__/stubs'
import { screen, waitFor } from '@testing-library/vue'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useUserStore } from '@/stores/userStore'
import { playbackService } from '@/services/QueuePlaybackService'
import Router from '@/router'
import { usePlaylistStore } from '@/stores/playlistStore'
import EditPlaylistForm from '@/components/playlist/EditPlaylistForm.vue'
import EditSmartPlaylistForm from '@/components/playlist/smart-playlist/EditSmartPlaylistForm.vue'

const openModalMock = vi.fn()

vi.mock('@/composables/useModal', () => ({
  useModal: () => ({
    openModal: openModalMock,
  }),
}))

import Component from './PlaylistContextMenu.vue'

describe('playlistContextMenu.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      useQueueStore().state.playables = []
      openModalMock.mockClear()
    },
  })

  const renderComponent = async (playlist: Playlist, user: CurrentUser | null = null) => {
    if (!vi.isMockFunction(usePlayableStore().fetchForPlaylist)) {
      h.mock(usePlayableStore(), 'fetchForPlaylist').mockResolvedValue([])
    }

    user = user || (h.factory('user').state('current').make() as CurrentUser)

    useUserStore().state.current = user

    const rendered = h.renderMenu(Component, {
      props: {
        playlist,
      },
    })

    await h.tick(2)

    return {
      ...rendered,
      playlist,
      user,
    }
  }

  const makeEditablePlaylist = (overrides: Partial<Playlist> = {}) =>
    h.factory('playlist').make({ permissions: { edit: true, delete: false }, ...overrides })

  const makeDeletablePlaylist = (overrides: Partial<Playlist> = {}) =>
    h.factory('playlist').make({ permissions: { edit: false, delete: true }, ...overrides })

  it('edits a standard playlist', async () => {
    const { playlist } = await renderComponent(makeEditablePlaylist())

    await h.user.click(screen.getByText('Edit…'))

    await assertOpenModal(openModalMock, EditPlaylistForm, { playlist })
  })

  it('edits a smart playlist', async () => {
    const { playlist } = await renderComponent(
      factory('playlist')
        .state('smart')
        .make({ permissions: { edit: true, delete: false } }),
    )

    await h.user.click(screen.getByText('Edit…'))

    await assertOpenModal(openModalMock, EditSmartPlaylistForm, { playlist })
  })

  it('deletes a playlist', async () => {
    const deleteMock = h.mock(usePlaylistStore(), 'delete')
    const { playlist } = await renderComponent(makeDeletablePlaylist())

    await h.user.click(screen.getByText('Delete'))

    expect(deleteMock).toHaveBeenCalledWith(playlist)
  })

  it('hides Edit when only delete is permitted', async () => {
    await renderComponent(makeDeletablePlaylist())

    expect(screen.queryByText('Edit…')).toBeNull()
  })

  it('hides Delete when only edit is permitted', async () => {
    await renderComponent(makeEditablePlaylist())

    expect(screen.queryByText('Delete')).toBeNull()
  })

  it('plays', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylist').mockResolvedValue(songs)
    const queueMock = h.mock(playbackService, 'queueAndPlay')
    const goMock = h.mock(Router, 'go')
    const { playlist } = await renderComponent(h.factory('playlist').make())

    await h.user.click(screen.getByText('Play'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlist)
      expect(queueMock).toHaveBeenCalledWith(songs)
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })

  it('offers nothing to play in an empty playlist', async () => {
    h.mock(usePlayableStore(), 'fetchForPlaylist').mockResolvedValue([])

    await renderComponent(h.factory('playlist').make())

    await waitFor(() => expect(screen.queryByText('Play')).toBeNull())
    expect(screen.queryByText('Shuffle')).toBeNull()
    expect(screen.queryByText('Add to queue')).toBeNull()
  })
  it('queues', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(usePlayableStore(), 'fetchForPlaylist').mockResolvedValue(songs)
    const queueMock = h.mock(useQueueStore(), 'queueAfterCurrent')
    const toastMock = h.mock(MessageToasterStub.value, 'success')
    const { playlist } = await renderComponent(h.factory('playlist').make())

    await h.user.click(screen.getByText('Add to queue'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(playlist)
      expect(queueMock).toHaveBeenCalledWith(songs)
      expect(toastMock).toHaveBeenCalledWith('Playlist added to queue.')
    })
  })

  it('does not have an option to edit or delete if the playlist is not owned by the current user', async () => {
    const playlist = h.factory('playlist').make({ permissions: { edit: false, delete: false } })
    await renderComponent(playlist, h.factory('user').state('current').make() as CurrentUser)

    expect(screen.queryByText('Edit…')).toBeNull()
    expect(screen.queryByText('Delete')).toBeNull()
  })
})
