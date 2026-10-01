import { defineStore } from 'pinia'
import { differenceBy, orderBy, unionBy, uniqBy } from 'lodash-es'
import type { Reactive } from 'vue'
import { reactive, watch } from 'vue'
import { useViewport } from '@/composables/useViewport'
import { arrayify, moveItemsInList, use } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { normalizeForComparison, secondsToHumanReadable } from '@/utils/formatters'
import { queryClient } from '@/services/queryClient'
import { http } from '@/services/http'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { useVault } from '@/composables/useVault'
import { dropFromListPages } from '@/composables/useListPages'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { useOverviewStore } from '@/stores/overviewStore'
import { usePlaylistStore } from '@/stores/playlistStore'

export interface SongUpdateData {
  title?: string
  artist_name?: string
  album_name?: string
  album_artist_name?: string
  track?: number | null
  disc?: number | null
  lyrics?: string
  year?: number | null
  genre?: string
  visibility?: 'public' | 'private' | 'unchanged'
}

export interface SongUpdateResult {
  songs: Song[]
  artists: Artist[]
  albums: Album[]
  removed: {
    album_ids: Album['id'][]
    artist_ids: Artist['id'][]
  }
}

export type SongListPaginateParams = PaginateParams<PlayableListSortField>
export type SongListCursorPaginateParams = CursorPaginateParams<PlayableListSortField>

const getFormattedLength = (playables: MaybeArray<Playable>) =>
  secondsToHumanReadable(arrayify(playables).reduce((total, p) => total + p.length, 0))

const matchSongsByTitle = (title: string, songs: Song[]) => {
  const normalizedTitle = normalizeForComparison(title)
  return songs.find(song => normalizeForComparison(song.title) === normalizedTitle) ?? null
}

const ensureNotDeleted = (songs: MaybeArray<Song>) => arrayify(songs).filter(({ deleted }) => !deleted)

/**
 * Increase the play count for a playable.
 */
const registerPlay = async (playable: Playable) => {
  // A Subsonic scrobble counts the play; koel's start time is in seconds.
  await subsonic.scrobble(playable.id, true, playable.play_start_time ? playable.play_start_time * 1000 : undefined)

  playable.play_count++
  playable.played_at = new Date().toISOString()
}

const getSourceUrl = (playable: Playable) => {
  const preferences = usePreferenceStore()

  return useViewport().isTouch.value && preferences.transcode_on_mobile
    ? subsonic.streamUrl(playable.id, preferences.transcode_quality)
    : subsonic.streamUrl(playable.id)
}

/** Songs (and other playables), each kept once and shared by every list showing it. */
export const usePlayableStore = defineStore('playable', () => {
  const { vault, syncWithVault } = useVault<Playable>({
    onItemAdded: playable => {
      playable.playback_state = 'Stopped'
      // Most played songs on Home follow the counts.
      watch(
        () => playable.play_count,
        () => useOverviewStore().refreshPlayStats(),
      )
    },
  })

  const state = reactive<{ favorites: Playable[] }>({
    favorites: [],
  })

  const findPlaying = () => {
    for (const playable of vault.values()) {
      if (playable.playback_state !== 'Stopped') {
        return playable
      }
    }

    return undefined
  }

  const byId = (id: Playable['id']) => {
    const playable = vault.get(id)

    if (!playable || playable.deleted) {
      return undefined
    }

    return playable
  }

  const byIds = <T extends Playable = Playable>(ids: T['id'][]) => {
    const playables: Playable[] = []
    ids.forEach(id => use(byId(id), song => playables.push(song!)))
    return playables as T[]
  }

  const byAlbum = (album: Album) =>
    Array.from(vault.values()).filter(playable => playable.album_id === album.id) as Song[]

  const syncAlbumProperties = (album: Album) => {
    byAlbum(album).forEach(a => {
      a.album_cover = album.cover
      a.album_name = album.name
    })
  }

  const byArtist = (artist: Artist) =>
    Array.from(vault.values()).filter(playable => playable.artist_id === artist.id) as Song[]

  const byAlbumArtist = (artist: Artist) =>
    Array.from(vault.values()).filter(playable => playable.album_artist_id === artist.id) as Song[]

  const syncArtistProperties = (artist: Artist) => {
    byArtist(artist).forEach(a => {
      a.artist_name = artist.name
    })

    byAlbumArtist(artist).forEach(a => {
      a.album_artist_name = artist.name
    })
  }

  const resolve = async (id: Playable['id']) => {
    let playable = byId(id)

    if (!playable) {
      try {
        playable = syncWithVault(await subsonic.song(id))[0]
      } catch (error: unknown) {
        logger.error(error)
      }
    }

    return playable
  }

  const updateSongs = async (songsToUpdate: Song[], data: SongUpdateData) => {
    const result = await http.put<SongUpdateResult>('songs', {
      data,
      songs: songsToUpdate.map(song => song.id),
    })

    syncWithVault(result.songs)

    const albumStore = useAlbumStore()
    const artistStore = useArtistStore()

    albumStore.syncWithVault(result.albums)
    artistStore.syncWithVault(result.artists)

    albumStore.removeByIds(result.removed.album_ids)
    artistStore.removeByIds(result.removed.artist_ids)

    return result
  }

  const fetchSongsForAlbum = async (album: Album | Album['id']) => {
    const id = typeof album === 'string' ? album : album.id

    return ensureNotDeleted(
      (await queryClient.fetchQuery({
        queryKey: ['album', id, 'songs'],
        queryFn: async () => syncWithVault(await subsonic.albumSongs(id)),
      })) as Song[],
    )
  }

  const invalidateAlbumAndArtistSongCaches = async (song: Song) => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['album', song.album_id, 'songs'] }),
      queryClient.invalidateQueries({ queryKey: ['artist', song.artist_id, 'songs'] }),
    ])
  }

  const fetchSongsForArtist = async (artist: Artist | Artist['id']) => {
    const id = typeof artist === 'string' ? artist : artist.id

    return ensureNotDeleted(
      (await queryClient.fetchQuery({
        queryKey: ['artist', id, 'songs'],
        queryFn: async () => syncWithVault((await library.songs({ artist: id, sort: 'album_name', limit: 500 })).items),
      })) as Song[],
    )
  }

  const fetchForPlaylist = async (playlist: Playlist | Playlist['id'], refresh = false) => {
    const id = typeof playlist === 'string' ? playlist : playlist.id

    const songs = ensureNotDeleted(
      (await queryClient.fetchQuery({
        queryKey: ['playlist', id, 'songs'],
        queryFn: async () => syncWithVault(await subsonic.playlistSongs(id)),
        // Asked for afresh: whatever was kept is old. (An undefined staleTime would mean always.)
        ...(refresh && { staleTime: 0 }),
      })) as Song[],
    )

    usePlaylistStore().byId(id)!.playables = songs

    return songs
  }

  const fetchForPlaylists = async (playlists: Playlist[]) => {
    const playables: Playable[] = []

    for await (const playlist of playlists) {
      playables.push(...(await fetchForPlaylist(playlist)))
    }

    return uniqBy(playables, 'id')
  }

  const paginateSongsByGenre = async (genre: Genre | Genre['id'], params: SongListCursorPaginateParams) => {
    const id = typeof genre === 'string' ? genre : genre.id

    const { items, nextCursor } = await library.songs({ ...params, genre: id })

    return { items: syncWithVault(items) as Song[], nextCursor }
  }

  const fetchSongsByGenre = async (genre: Genre | Genre['id'], random = false, limit = 500) => {
    const id = typeof genre === 'string' ? genre : genre.id

    return syncWithVault(
      random
        ? await subsonic.randomSongs(limit, id)
        : (await library.songs({ genre: id, sort: 'album_name', limit })).items,
    )
  }

  /** A page of all the songs (see useListPages). */
  const paginateSongs = async (params: SongListCursorPaginateParams) => {
    const { items, nextCursor } = await library.songs(params)

    return { items: syncWithVault(items), nextCursor }
  }

  const getMostPlayedSongs = (count: number) =>
    orderBy(
      Array.from(vault.values()).filter(playable => !playable.deleted && playable.play_count > 0),
      'play_count',
      'desc',
    ).slice(0, count) as Song[]

  const deleteSongsFromFilesystem = async (songs: Song[]) => {
    const ids = songs.map(song => {
      // Whenever a vault sync is requested (e.g., upon playlist/album/artist fetching)
      // songs marked as "deleted" will be excluded.
      song.deleted = true
      return song.id
    })

    await http.delete('songs', { songs: ids })

    // Gone from the lists kept, too: all songs, and each genre's.
    dropFromListPages(['songs'], ids)
    dropFromListPages(['genre'], ids)
  }

  const fetchFavorites = async () => {
    state.favorites = syncWithVault(await subsonic.starredSongs())
    return state.favorites
  }

  const toggleFavorite = async (playable: Reactive<Playable>) => {
    // Don't wait for the HTTP response to update the status, just toggle right away.
    // We'll update the liked status again after the HTTP request.
    playable.favorite = !playable.favorite

    try {
      await (playable.favorite ? subsonic.star([playable.id]) : subsonic.unstar([playable.id]))
    } catch (error) {
      playable.favorite = !playable.favorite
      throw error
    }

    state.favorites = playable.favorite
      ? unionBy(state.favorites, arrayify(playable), 'id')
      : differenceBy(state.favorites, arrayify(playable), 'id')
  }

  const rate = async (song: Reactive<Song>, rating: number) => {
    const previous = song.rating
    song.rating = rating

    try {
      await subsonic.setRating(song.id, rating)
    } catch (e) {
      song.rating = previous
      throw e
    }
  }

  const favorite = async (playables: MaybeArray<Playable>) => {
    playables = arrayify(playables)
    playables.forEach(playable => (playable.favorite = true))

    await subsonic.star(playables.map(playable => playable.id))

    state.favorites = unionBy(state.favorites, playables, 'id')
  }

  const undoFavorite = async (playables: MaybeArray<Playable>) => {
    playables = arrayify(playables)
    playables.forEach(playable => (playable.favorite = false))

    await subsonic.unstar(playables.map(playable => playable.id))

    state.favorites = differenceBy(state.favorites, playables, 'id')
  }

  // Stars have no order on the server, so a new order lasts until reload.
  const moveFavoritesInList = async (playables: MaybeArray<Playable>, target: Playable, placement: Placement) => {
    state.favorites.splice(0, state.favorites.length, ...moveItemsInList(state.favorites, playables, target, placement))
  }

  return {
    state,
    vault,
    syncWithVault,
    getFormattedLength,
    findPlaying,
    byId,
    byIds,
    byAlbum,
    syncAlbumProperties,
    byArtist,
    byAlbumArtist,
    syncArtistProperties,
    resolve,
    matchSongsByTitle,
    registerPlay,
    updateSongs,
    getSourceUrl,
    ensureNotDeleted,
    fetchSongsForAlbum,
    invalidateAlbumAndArtistSongCaches,
    fetchSongsForArtist,
    fetchForPlaylist,
    fetchForPlaylists,
    paginateSongsByGenre,
    fetchSongsByGenre,
    paginateSongs,
    getMostPlayedSongs,
    deleteSongsFromFilesystem,
    fetchFavorites,
    toggleFavorite,
    rate,
    favorite,
    undoFavorite,
    moveFavoritesInList,
  }
})
