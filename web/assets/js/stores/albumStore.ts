import { defineStore } from 'pinia'
import type { Reactive } from 'vue'
import { queryClient } from '@/services/queryClient'
import { huntingService } from '@/services/huntingService'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { logger } from '@/utils/logger'
import { useVault } from '@/composables/useVault'
import { dropFromListPages } from '@/composables/useListPages'
import { usePlayableStore } from '@/stores/playableStore'

const UNKNOWN_ALBUM_NAME = 'Unknown Album'

export interface AlbumUpdateData {
  title: string
  artist: string
  year: number | null
  tracks: Array<{ id: Song['id']; title: string; track: number | null }>
}

interface AlbumListPaginateParams extends CursorPaginateParams<AlbumListSortField> {
  favorites_only: boolean
}

const isUnknown = (album: Album | Album['name']) =>
  (typeof album === 'string' ? album : album.name) === UNKNOWN_ALBUM_NAME

export const useAlbumStore = defineStore('album', () => {
  const { vault, byId, syncWithVault } = useVault<Album>()

  const removeByIds = (ids: Album['id'][]) => {
    ids.forEach(id => {
      vault.delete(id)
      queryClient.removeQueries({ queryKey: ['album', id] })
    })
    dropFromListPages(['albums'], ids)
  }

  /** Changes the album's tags; píxiū moves its files to match. */
  const update = async (album: Album, data: AlbumUpdateData) => {
    await huntingService.editAlbum(album, data)

    await queryClient.invalidateQueries({ queryKey: ['album', album.id] })
    const updated = syncWithVault(await subsonic.album(album.id))

    const songStore = usePlayableStore()
    songStore.syncWithVault(await subsonic.albumSongs(album.id))
    songStore.syncAlbumProperties(updated[0])
  }

  const resolve = async (id: Album['id']) => {
    let album = byId(id)

    if (!album) {
      try {
        album = syncWithVault(
          await queryClient.fetchQuery({ queryKey: ['album', id], queryFn: () => subsonic.album(id) }),
        )[0]
      } catch (error: unknown) {
        logger.error(error)
      }
    }

    return album
  }

  /**
   * Fetch the (blurry) thumbnail-sized version of an album's cover.
   */
  const fetchThumbnail = async (id: Album['id']) => (await resolve(id))?.thumbnail ?? null

  /** A page of the album list (see useListPages). */
  const paginate = async (params: AlbumListPaginateParams) => {
    const { items, nextCursor } = await library.albums(params)

    return { items: syncWithVault(items), nextCursor }
  }

  const fetchForArtist = async (artist: Artist | Artist['id']) => {
    const id = typeof artist === 'string' ? artist : artist.id

    return syncWithVault(
      await queryClient.fetchQuery({ queryKey: ['artist', id, 'albums'], queryFn: () => subsonic.artistAlbums(id) }),
    )
  }

  const toggleFavorite = async (album: Reactive<Album>) => {
    // Don't wait for the HTTP response to update the status, just toggle right away.
    // We'll update the liked status again after the HTTP request.
    album.favorite = !album.favorite

    try {
      await (album.favorite ? subsonic.star([album.id]) : subsonic.unstar([album.id]))
    } catch (error) {
      album.favorite = !album.favorite
      throw error
    }
  }

  const rate = async (album: Reactive<Album>, rating: number) => {
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
  }

  return {
    vault,
    byId,
    syncWithVault,
    removeByIds,
    isUnknown,
    update,
    fetchThumbnail,
    resolve,
    paginate,
    fetchForArtist,
    toggleFavorite,
    rate,
  }
})
