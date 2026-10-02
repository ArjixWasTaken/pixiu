import type { Ref } from 'vue'
import { useQueueStore } from '@/stores/queueStore'
import { defineAsyncComponent } from '@/utils/helpers'
import { usePlaylistContentManagement } from '@/composables/usePlaylistContentManagement'
import { useModal } from '@/composables/useModal'
import { usePlayableStore } from '@/stores/playableStore'
const CreatePlaylistForm = defineAsyncComponent(() => import('@/components/playlist/CreatePlaylistForm.vue'))

export const usePlayableMenuMethods = (playables: Ref<Playable[]>, close: Closure) => {
  const { addToPlaylist } = usePlaylistContentManagement()
  const { openModal } = useModal()

  const trigger = async (cb: Closure) => {
    close()
    await cb()
  }

  return {
    queueAfterCurrent: () => trigger(() => useQueueStore().queueAfterCurrent(playables.value)),
    queueToBottom: () => trigger(() => useQueueStore().queue(playables.value)),
    addToFavorites: () => trigger(() => usePlayableStore().favorite(playables.value)),
    removeFromFavorites: () => trigger(() => usePlayableStore().undoFavorite(playables.value)),
    removeFromQueue: () => trigger(() => useQueueStore().unqueue(playables.value)),
    addToExistingPlaylist: (playlist: Playlist) => trigger(() => addToPlaylist(playlist, playables.value)),
    addToNewPlaylist: () =>
      trigger(() =>
        openModal<'CREATE_PLAYLIST_FORM'>(CreatePlaylistForm, { folder: null, playables: playables.value }),
      ),
  }
}
