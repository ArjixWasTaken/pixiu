import { describe, expect, it, vi } from 'vite-plus/test'
import { ref, shallowRef } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { assertOpenModal } from '@/__tests__/assertions'
import factory from '@/__tests__/factory'
import { ContextMenuKey } from '@/config/symbols'
import { arrayify } from '@/utils/helpers'
import { screen } from '@testing-library/vue'
import { downloadService } from '@/services/downloadService'
import { playbackService } from '@/services/QueuePlaybackService'
import { commonStore } from '@/stores/commonStore'
import { playlistStore } from '@/stores/playlistStore'
import { queueStore } from '@/stores/queueStore'
import { playableStore } from '@/stores/playableStore'
import { MessageToasterStub } from '@/__tests__/stubs'
import Router from '@/router'
import CreatePlaylistForm from '@/components/playlist/CreatePlaylistForm.vue'

const openModalMock = vi.fn()

vi.mock('@/composables/useModal', () => ({
  useModal: () => ({
    openModal: openModalMock,
  }),
}))

const makeAvailableOfflineMock = vi.fn()
const removeOfflineCacheMock = vi.fn()
const isCachedMock = vi.fn().mockReturnValue(false)

vi.mock('@/composables/useOfflinePlayback', () => ({
  useOfflinePlayback: () => ({
    swReady: ref(true),
    makeAvailableOffline: makeAvailableOfflineMock,
    removeOfflineCache: removeOfflineCacheMock,
    isCached: isCachedMock,
  }),
}))

import Component from './PlayableContextMenu.vue'

// Mock service worker controller so offline menu items show up
Object.defineProperty(navigator, 'serviceWorker', {
  value: { controller: { postMessage: vi.fn() }, addEventListener: vi.fn() },
  writable: true,
  configurable: true,
})

describe('playableContextMenu.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      queueStore.state.playables = []
      openModalMock.mockClear()
      makeAvailableOfflineMock.mockClear()
      removeOfflineCacheMock.mockClear()
      isCachedMock.mockReturnValue(false)
    },
  })

  const renderComponent = async (playables?: MaybeArray<Playable>) => {
    playables = playables ? arrayify(playables) : h.factory('song').make(5)

    const rendered = h.render(Component, {
      props: {
        playables,
      },
    })

    await h.tick(2)

    return {
      ...rendered,
      playables,
    }
  }

  const fillQueue = () => {
    queueStore.state.playables = h.factory('song').make(5)
    playableStore.syncWithVault(queueStore.state.playables)
    queueStore.state.playables[2].playback_state = 'Playing'
  }

  it('plays', async () => {
    h.createAudioPlayer()

    const playMock = h.mock(playbackService, 'play')
    const song = h.factory('song').make({ playback_state: 'Stopped' })
    await renderComponent(song)

    await h.user.click(screen.getByText('Play'))

    expect(playMock).toHaveBeenCalledWith(song)
  })

  it('pauses playback', async () => {
    h.createAudioPlayer()

    const pauseMock = h.mock(playbackService, 'pause')
    await renderComponent(h.factory('song').make({ playback_state: 'Playing' }))

    await h.user.click(screen.getByText('Pause'))

    expect(pauseMock).toHaveBeenCalled()
  })

  it('resumes playback', async () => {
    h.createAudioPlayer()

    const resumeMock = h.mock(playbackService, 'resume')
    await renderComponent(h.factory('song').make({ playback_state: 'Paused' }))

    await h.user.click(screen.getByText('Play'))

    expect(resumeMock).toHaveBeenCalled()
  })

  it('goes to album details screen', async () => {
    const goMock = h.mock(Router, 'go')
    const song = h.factory('song').make()
    await renderComponent(song)

    await h.user.click(screen.getByText(`Album: ${song.album_name}`))

    expect(goMock).toHaveBeenCalledWith(`/#/albums/${song.album_id}`)
  })

  it('goes to artist details screen', async () => {
    const goMock = h.mock(Router, 'go')
    const song = h.factory('song').make()
    await renderComponent(song)

    await h.user.click(screen.getByText(`Artist: ${song.artist_name}`))

    expect(goMock).toHaveBeenCalledWith(`/#/artists/${song.artist_id}`)
  })

  it('downloads', async () => {
    const downloadMock = h.mock(downloadService, 'fromPlayables')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Download'))

    expect(downloadMock).toHaveBeenCalledWith(playables)
  })

  it('queues', async () => {
    const queueMock = h.mock(queueStore, 'queue')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Queue'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('queues after current', async () => {
    fillQueue()
    const queueMock = h.mock(queueStore, 'queueAfterCurrent')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('After current song'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('queues to bottom', async () => {
    fillQueue()
    const queueMock = h.mock(queueStore, 'queue')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Bottom of queue'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('queues to top', async () => {
    fillQueue()
    const queueMock = h.mock(queueStore, 'queueToTop')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Top of queue'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('removes from queue', async () => {
    fillQueue()
    const removeMock = h.mock(queueStore, 'unqueue')

    h.visit('/queue')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Remove from queue'))

    expect(removeMock).toHaveBeenCalledWith(playables)
  })

  it('does not show "Remove from queue" when not on Queue screen', async () => {
    fillQueue()

    h.visit('/songs')
    await renderComponent()

    expect(screen.queryByText('Remove from queue')).toBeNull()
  })

  it('adds to favorites', async () => {
    const likeMock = h.mock(playableStore, 'favorite')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Favorites'))

    expect(likeMock).toHaveBeenCalledWith(playables)
  })

  it('does not have an option to add to favorites for Favorites screen', async () => {
    h.visit('/favorites')
    await renderComponent()

    expect(screen.queryByText('Favorites')).toBeNull()
  })

  it('removes from favorites', async () => {
    const unlikeMock = h.mock(playableStore, 'undoFavorite')

    h.visit('/favorites')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Remove from favorites'))

    expect(unlikeMock).toHaveBeenCalledWith(playables)
  })

  it('lists and adds to existing playlist', async () => {
    playlistStore.state.playlists = h.factory('playlist').make(3)
    playlistStore.state.playlists.forEach(playlist => (playlist.permissions = { edit: true, delete: true }))
    const addMock = h.mock(playlistStore, 'addContent')
    h.mock(MessageToasterStub.value, 'success')
    const { playables } = await renderComponent()

    playlistStore.state.playlists.forEach(playlist => screen.queryByText(playlist.name))

    await h.user.click(screen.getByText(playlistStore.state.playlists[0].name))

    expect(addMock).toHaveBeenCalledWith(playlistStore.state.playlists[0], playables)
  })

  it('does not list mirrors of watched playlists', async () => {
    playlistStore.state.playlists = [
      h.factory('playlist').make({ name: 'My Mirror', permissions: { edit: false, delete: false } }),
    ]

    await renderComponent()

    expect(screen.queryByText('My Mirror')).toBeNull()
  })

  it('does not list smart playlists', async () => {
    playlistStore.state.playlists = h.factory('playlist').make(3)
    playlistStore.state.playlists.push(factory('playlist').state('smart').make({ name: 'My Smart Playlist' }))

    await renderComponent()

    expect(screen.queryByText('My Smart Playlist')).toBeNull()
  })

  it('does not have an option to remove from playlist if not on Playlist screen', async () => {
    h.visit('/songs')
    await renderComponent()

    expect(screen.queryByText('Remove from playlist')).toBeNull()
  })

  it('does not allow edit songs if current user is not admin', async () => {
    h.actingAsUser()
    await renderComponent()
    expect(screen.queryByText('Edit…')).toBeNull()
  })

  it('does not have an option to delete songs if current user is not admin', async () => {
    h.actingAsUser()
    await renderComponent()
    expect(screen.queryByText('Delete from Filesystem')).toBeNull()
  })

  it('creates playlist from selected songs', async () => {
    h.actingAsUser()
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('New playlist…'))

    await assertOpenModal(openModalMock, CreatePlaylistForm, { folder: null, playables })
  })

  it('makes songs available offline', async () => {
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Make available offline'))

    for (const playable of playables) {
      expect(makeAvailableOfflineMock).toHaveBeenCalledWith(playable)
    }
  })

  it('removes offline versions when all songs are cached', async () => {
    isCachedMock.mockReturnValue(true)
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Remove offline copies'))

    for (const playable of playables) {
      expect(removeOfflineCacheMock).toHaveBeenCalledWith(playable)
    }
  })

  it('links to MusicBrainz', async () => {
    commonStore.state.uses_musicbrainz = true
    const openMock = h.mock(window, 'open')
    const song = h.factory('song').make()

    await renderComponent(song)
    await h.user.click(screen.getByText('View on MusicBrainz'))

    expect(openMock).toHaveBeenCalledWith(`https://musicbrainz.org/recording/${song.mbid}`, '_blank')
  })

  it('does not link to MusicBrainz when MusicBrainz is disabled', async () => {
    commonStore.state.uses_musicbrainz = false
    await renderComponent(h.factory('song').make())

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('does not link to MusicBrainz when the song has no identifier', async () => {
    commonStore.state.uses_musicbrainz = true
    await renderComponent(h.factory('song').make({ mbid: null }))

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('does not link to MusicBrainz when multiple playables are selected', async () => {
    commonStore.state.uses_musicbrainz = true
    await renderComponent(h.factory('song').make(3))

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('closes the menu after rating', async () => {
    h.mock(playableStore, 'rate')
    const menu = shallowRef<any>({ component: Component, position: { top: 0, left: 0 } })
    const song = h.factory('song').make({ rating: 0 })

    h.render(Component, {
      props: { playables: [song] },
      global: { provide: { [ContextMenuKey as symbol]: menu } },
    })

    await h.tick(2)
    await h.user.click(screen.getByRole('radio', { name: 'Rate 4 of 5' }))

    expect(menu.value.component).toBeNull()
  })
})
