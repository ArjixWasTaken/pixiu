import { describe, expect, it, vi } from 'vite-plus/test'
import { ref } from 'vue'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { createHarness } from '@/__tests__/TestHarness'
import { assertOpenModal } from '@/__tests__/assertions'
import CreatePlaylistForm from '@/components/playlist/CreatePlaylistForm.vue'

const openModalMock = vi.fn()

vi.mock('@/composables/useModal', () => ({
  useModal: () => ({
    openModal: openModalMock,
  }),
}))

vi.mock('@/composables/usePlaylistContentManagement', () => ({
  usePlaylistContentManagement: () => ({
    addToPlaylist: vi.fn(),
  }),
}))

import { usePlayableMenuMethods } from './usePlayableMenuMethods'

describe('usePlayableMenuMethods', () => {
  const h = createHarness({
    beforeEach: () => openModalMock.mockClear(),
  })

  it('queues to bottom', async () => {
    const songs = [h.factory('song').make()]
    const playables = ref<Playable[]>(songs)
    const close = vi.fn()

    h.mock(useQueueStore(), 'queue')

    const { queueToBottom } = usePlayableMenuMethods(playables, close)
    await queueToBottom()

    expect(close).toHaveBeenCalled()
    expect(useQueueStore().queue).toHaveBeenCalledWith(songs)
  })

  it('adds to favorites', async () => {
    const songs = [h.factory('song').make()]
    const playables = ref<Playable[]>(songs)
    const close = vi.fn()

    h.mock(usePlayableStore(), 'favorite')

    const { addToFavorites } = usePlayableMenuMethods(playables, close)
    await addToFavorites()

    expect(usePlayableStore().favorite).toHaveBeenCalledWith(songs)
  })

  it('opens new playlist modal', async () => {
    const songs = [h.factory('song').make()]
    const playables = ref<Playable[]>(songs)
    const close = vi.fn()

    const { addToNewPlaylist } = usePlayableMenuMethods(playables, close)
    await addToNewPlaylist()

    await assertOpenModal(openModalMock, CreatePlaylistForm, { folder: null, playables: songs })
  })
})
