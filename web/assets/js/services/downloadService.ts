import { subsonic } from '@/services/subsonic'
import { zipDownloadService } from '@/services/zipDownloadService'
import { usePlayableStore } from '@/stores/playableStore'
import { arrayify } from '@/utils/helpers'

export const downloadService = {
  async fromPlayables(playables: MaybeArray<Playable>) {
    const items = arrayify(playables)

    if (items.length === 1) {
      this.trigger(items[0])
      return
    }

    await zipDownloadService.start(items, 'pixiu-download', 'none')
  },

  async fromAlbum(album: Album) {
    await zipDownloadService.start(await usePlayableStore().fetchSongsForAlbum(album), album.name, 'track')
  },

  async fromArtist(artist: Artist) {
    await zipDownloadService.start(await usePlayableStore().fetchSongsForArtist(artist), artist.name, 'none')
  },

  async fromPlaylist(playlist: Playlist) {
    await zipDownloadService.start(await usePlayableStore().fetchForPlaylist(playlist), playlist.name, 'position')
  },

  async fromFavorites() {
    if (!usePlayableStore().state.favorites.length) {
      return
    }

    await zipDownloadService.start(usePlayableStore().state.favorites, 'Favorites', 'none')
  },

  trigger: (playable: Playable) => {
    open(subsonic.url('download', { id: playable.id }))
  },
}
