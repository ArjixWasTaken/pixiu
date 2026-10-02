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
import { useCommonStore } from '@/stores/commonStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
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
      useQueueStore().state.playables = []
      openModalMock.mockClear()
      makeAvailableOfflineMock.mockClear()
      removeOfflineCacheMock.mockClear()
      isCachedMock.mockReturnValue(false)
    },
  })

  const renderComponent = async (playables?: MaybeArray<Playable>) => {
    playables = playables ? arrayify(playables) : h.factory('song').make(5)

    const rendered = h.renderMenu(Component, {
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
    useQueueStore().state.playables = h.factory('song').make(5)
    usePlayableStore().syncWithVault(useQueueStore().state.playables)
    useQueueStore().state.playables[2].playback_state = 'Playing'
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

    await h.user.click(screen.getByText('Go to'))
    await h.user.click(screen.getByText(`Album: ${song.album_name}`))

    expect(goMock).toHaveBeenCalledWith(`/albums/${song.album_id}`)
  })

  it('goes to artist details screen', async () => {
    const goMock = h.mock(Router, 'go')
    const song = h.factory('song').make()
    await renderComponent(song)

    await h.user.click(screen.getByText('Go to'))
    await h.user.click(screen.getByText(`Artist: ${song.artist_name}`))

    expect(goMock).toHaveBeenCalledWith(`/artists/${song.artist_id}`)
  })

  it('downloads', async () => {
    const downloadMock = h.mock(downloadService, 'fromPlayables')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Download'))

    expect(downloadMock).toHaveBeenCalledWith(playables)
  })

  it('adds to the queue', async () => {
    const queueMock = h.mock(useQueueStore(), 'queue')
    const { playables } = await renderComponent()

    // With nothing playing, nothing to play next.
    expect(screen.queryByText('Play next')).toBeNull()
    await h.user.click(screen.getByText('Add to queue'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('plays next', async () => {
    fillQueue()
    const queueMock = h.mock(useQueueStore(), 'queueAfterCurrent')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Play next'))

    expect(queueMock).toHaveBeenCalledWith(playables)
  })

  it('has separators only between groups', async () => {
    await renderComponent()

    const items = [...document.querySelector('[role=menu] > ul')!.children]
    const separators = items.map(item => item.getAttribute('role') === 'separator')
    expect(separators).toContain(true)

    expect(separators[0]).toBe(false)
    expect(separators.at(-1)).toBe(false)
    expect(separators.some((separator, i) => separator && separators[i + 1])).toBe(false)
  })

  it('removes from queue', async () => {
    fillQueue()
    const removeMock = h.mock(useQueueStore(), 'unqueue')

    await h.visit('/queue')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Remove from queue'))

    expect(removeMock).toHaveBeenCalledWith(playables)
  })

  it('does not show "Remove from queue" when not on Queue screen', async () => {
    fillQueue()

    await h.visit('/songs')
    await renderComponent()

    expect(screen.queryByText('Remove from queue')).toBeNull()
  })

  it('adds to favorites', async () => {
    const likeMock = h.mock(usePlayableStore(), 'favorite')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Add to'))
    await h.user.click(screen.getByText('Favorites'))

    expect(likeMock).toHaveBeenCalledWith(playables)
  })

  it('does not have an option to add to favorites for Favorites screen', async () => {
    await h.visit('/favorites')
    await renderComponent()

    await h.user.click(screen.getByText('Add to'))
    expect(screen.queryByText('Favorites')).toBeNull()
  })

  it('removes from favorites', async () => {
    const unlikeMock = h.mock(usePlayableStore(), 'undoFavorite')

    await h.visit('/favorites')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Remove from favorites'))

    expect(unlikeMock).toHaveBeenCalledWith(playables)
  })

  it('lists and adds to existing playlist', async () => {
    usePlaylistStore().state.playlists = h.factory('playlist').make(3)
    usePlaylistStore().state.playlists.forEach(playlist => (playlist.permissions = { edit: true, delete: true }))
    const addMock = h.mock(usePlaylistStore(), 'addContent')
    h.mock(MessageToasterStub.value, 'success')
    const { playables } = await renderComponent()

    await h.user.click(screen.getByText('Add to'))
    usePlaylistStore().state.playlists.forEach(playlist => screen.getByText(playlist.name))

    await h.user.click(screen.getByText(usePlaylistStore().state.playlists[0].name))

    expect(addMock).toHaveBeenCalledWith(usePlaylistStore().state.playlists[0], playables)
  })

  it('does not list mirrors of watched playlists', async () => {
    usePlaylistStore().state.playlists = [
      h.factory('playlist').make({ name: 'My Mirror', permissions: { edit: false, delete: false } }),
    ]

    await renderComponent()

    await h.user.click(screen.getByText('Add to'))
    expect(screen.queryByText('My Mirror')).toBeNull()
  })

  it('does not list smart playlists', async () => {
    usePlaylistStore().state.playlists = h.factory('playlist').make(3)
    usePlaylistStore().state.playlists.push(factory('playlist').state('smart').make({ name: 'My Smart Playlist' }))

    await renderComponent()

    await h.user.click(screen.getByText('Add to'))
    expect(screen.queryByText('My Smart Playlist')).toBeNull()
  })

  it('does not have an option to remove from playlist if not on Playlist screen', async () => {
    await h.visit('/songs')
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

    await h.user.click(screen.getByText('Add to'))
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
    useCommonStore().state.uses_musicbrainz = true
    const openMock = h.mock(window, 'open')
    const song = h.factory('song').make()

    await renderComponent(song)
    await h.user.click(screen.getByText('View on MusicBrainz'))

    expect(openMock).toHaveBeenCalledWith(`https://musicbrainz.org/recording/${song.mbid}`, '_blank')
  })

  it('does not link to MusicBrainz when MusicBrainz is disabled', async () => {
    useCommonStore().state.uses_musicbrainz = false
    await renderComponent(h.factory('song').make())

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('does not link to MusicBrainz when the song has no identifier', async () => {
    useCommonStore().state.uses_musicbrainz = true
    await renderComponent(h.factory('song').make({ mbid: null }))

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('does not link to MusicBrainz when multiple playables are selected', async () => {
    useCommonStore().state.uses_musicbrainz = true
    await renderComponent(h.factory('song').make(3))

    expect(screen.queryByText('View on MusicBrainz')).toBeNull()
  })

  it('closes the menu after rating', async () => {
    h.mock(usePlayableStore(), 'rate')
    const menu = shallowRef<any>({ component: Component, position: { top: 0, left: 0 } })
    const song = h.factory('song').make({ rating: 0 })

    h.renderMenu(Component, {
      props: { playables: [song] },
      global: { provide: { [ContextMenuKey as symbol]: menu } },
    })

    await h.tick(2)
    await h.user.click(screen.getByRole('radio', { name: 'Rate 4 of 5' }))

    expect(menu.value.component).toBeNull()
  })
})
