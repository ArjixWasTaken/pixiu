import { usePlaylistStore } from '@/stores/playlistStore'
import { eventBus } from '@/utils/eventBus'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

export const usePlaylistContentManagement = () => {
  const { handleHttpError } = useErrorHandler('dialog')
  const { toastSuccess } = useMessageToaster()

  const songs = (playables: Playable[]) => (playables.length === 1 ? '1 song' : `${playables.length} songs`)

  const addToPlaylist = async (playlist: Playlist, playables: Playable[]) => {
    if (playlist.is_smart || playables.length === 0) {
      return
    }

    try {
      await usePlaylistStore().addContent(playlist, playables)
      eventBus.emit('PLAYLIST_UPDATED', playlist)
      toastSuccess(`Added ${songs(playables)} to “${playlist.name}”.`)
    } catch (error: unknown) {
      handleHttpError(error)
    }
  }

  const removeFromPlaylist = async (playlist: Playlist, playables: Playable[]) => {
    if (playlist.is_smart) {
      return
    }

    try {
      await usePlaylistStore().removeContent(playlist, playables)
      eventBus.emit('PLAYLIST_CONTENT_REMOVED', { playlist, playables })
      toastSuccess(`Removed ${songs(playables)} from “${playlist.name}”.`)
    } catch (error: unknown) {
      handleHttpError(error)
    }
  }

  return {
    addToPlaylist,
    removeFromPlaylist,
  }
}
