import type { Reactive } from 'vue'
import { reactive } from 'vue'
import { differenceBy, unionBy } from 'lodash-es'
import { cache } from '@/services/cache'
import { http } from '@/services/http'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { logger } from '@/utils/logger'
import { useVault } from '@/composables/useVault'
import { playableStore as songStore } from '@/stores/playableStore'

const UNKNOWN_ALBUM_NAME = 'Unknown Album'

export interface AlbumUpdateData {
  name: Album['name']
  year: Album['year']
  cover?: Album['cover'] | null
}

interface AlbumListPaginateParams extends CursorPaginateParams<AlbumListSortField> {
  favorites_only: boolean
}

export const albumStore = {
  ...useVault<Album>(),

  state: reactive({
    albums: [] as Album[],
  }),

  removeByIds(ids: Album['id'][]) {
    this.state.albums = differenceBy(
      this.state.albums,
      ids.map(id => this.byId(id)),
      'id',
    )
    ids.forEach(id => {
      this.vault.delete(id)
      cache.remove(['album', id])
    })
  },

  isUnknown: (album: Album | Album['name']) => {
    if (typeof album === 'string') {
      return album === UNKNOWN_ALBUM_NAME
    }

    return album.name === UNKNOWN_ALBUM_NAME
  },

  async update(album: Album, data: AlbumUpdateData) {
    const updated = await http.put<Album>(`albums/${album.id}`, data)
    this.state.albums = unionBy(this.state.albums, this.syncWithVault(updated), 'id')

    songStore.syncAlbumProperties(album)
  },

  /**
   * Fetch the (blurry) thumbnail-sized version of an album's cover.
   */
  async fetchThumbnail(id: Album['id']) {
    return (await this.resolve(id))?.thumbnail ?? null
  },

  async resolve(id: Album['id']) {
    let album = this.byId(id)

    if (!album) {
      try {
        album = this.syncWithVault(
          await cache.remember(['album', id], async () => await subsonic.album(id)),
        )[0]
      } catch (error: unknown) {
        logger.error(error)
      }
    }

    return album
  },

  async paginate(params: AlbumListPaginateParams) {
    const { items, nextCursor } = await library.albums(params)
    this.state.albums = unionBy(this.state.albums, this.syncWithVault(items), 'id')

    return nextCursor
  },

  async fetchForArtist(artist: Artist | Artist['id']) {
    const id = typeof artist === 'string' ? artist : artist.id

    return this.syncWithVault(
      await cache.remember(['artist-albums', id], async () => await subsonic.artistAlbums(id)),
    )
  },

  async toggleFavorite(album: Reactive<Album>) {
    // Don't wait for the HTTP response to update the status, just toggle right away.
    // We'll update the liked status again after the HTTP request.
    album.favorite = !album.favorite

    try {
      await (album.favorite ? subsonic.star([album.id]) : subsonic.unstar([album.id]))
    } catch (error) {
      album.favorite = !album.favorite
      throw error
    }
  },

  async rate(album: Reactive<Album>, rating: number) {
    const previous = album.rating
    album.rating = rating

    try {
      await subsonic.setRating(album.id, rating)
    } catch (error) {
      if (album.rating === rating) {
        album.rating = previous
      }

      throw error
    }
  },

  reset() {
    this.vault.clear()
    this.state.albums = []
  },
}
