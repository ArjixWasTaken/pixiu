import { defineStore } from 'pinia'
import type { Reactive } from 'vue'
import { queryClient } from '@/services/queryClient'
import { http } from '@/services/http'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { logger } from '@/utils/logger'
import { useVault } from '@/composables/useVault'
import { dropFromListPages } from '@/composables/useListPages'
import { usePlayableStore } from '@/stores/playableStore'

const UNKNOWN_ARTIST_NAME = 'Unknown Artist'
const VARIOUS_ARTISTS_NAME = 'Various Artists'

export interface ArtistUpdateData {
  name: Artist['name']
  image?: Artist['image'] | null
}

interface ArtistListPaginateParams extends CursorPaginateParams<ArtistListSortField> {
  favorites_only: boolean
}

const isVarious = (artist: Artist | Artist['name']) =>
  (typeof artist === 'string' ? artist : artist.name) === VARIOUS_ARTISTS_NAME

const isUnknown = (artist: Artist | Artist['name']) =>
  (typeof artist === 'string' ? artist : artist.name) === UNKNOWN_ARTIST_NAME

const isStandard = (artist: Artist | Artist['name']) => !isVarious(artist) && !isUnknown(artist)

export const useArtistStore = defineStore('artist', () => {
  const { vault, byId, syncWithVault } = useVault<Artist>()

  const removeByIds = (ids: Artist['id'][]) => {
    ids.forEach(id => {
      vault.delete(id)
      queryClient.removeQueries({ queryKey: ['artist', id] })
    })
    dropFromListPages(['artists'], ids)
  }

  const update = async (artist: Artist, data: ArtistUpdateData) => {
    const updated = await http.put<Artist>(`artists/${artist.id}`, data)
    syncWithVault(updated)
    usePlayableStore().syncArtistProperties(updated)
  }

  const resolve = async (id: Artist['id']) => {
    let artist = byId(id)

    if (!artist) {
      try {
        artist = syncWithVault(
          await queryClient.fetchQuery({ queryKey: ['artist', id], queryFn: () => subsonic.artist(id) }),
        )[0]
      } catch (error: unknown) {
        logger.error(error)
      }
    }

    return artist
  }

  /** A page of the artist list (see useListPages). */
  const paginate = async (params: ArtistListPaginateParams) => {
    const { items, nextCursor } = await library.artists(params)

    return { items: syncWithVault(items), nextCursor }
  }

  const toggleFavorite = async (artist: Reactive<Artist>) => {
    // Don't wait for the HTTP response to update the status, just toggle right away.
    // We'll update the liked status again after the HTTP request.
    artist.favorite = !artist.favorite

    try {
      await (artist.favorite ? subsonic.star([artist.id]) : subsonic.unstar([artist.id]))
    } catch (error) {
      artist.favorite = !artist.favorite
      throw error
    }
  }

  const rate = async (artist: Reactive<Artist>, rating: number) => {
    const previous = artist.rating
    artist.rating = rating

    try {
      await subsonic.setRating(artist.id, rating)
    } catch (error) {
      if (artist.rating === rating) {
        artist.rating = previous
      }

      throw error
    }
  }

  return {
    vault,
    byId,
    syncWithVault,
    removeByIds,
    isVarious,
    isUnknown,
    isStandard,
    update,
    resolve,
    paginate,
    toggleFavorite,
    rate,
  }
})
