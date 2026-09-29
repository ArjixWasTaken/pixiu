import { cache } from '@/services/cache'
import { subsonic } from '@/services/subsonic'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'

export const encyclopediaService = {
  async fetchForArtist(artist: Artist) {
    artist = artistStore.syncWithVault(artist)[0]
    const cacheKey = ['artist.info', artist.id]

    if (cache.has(cacheKey)) {
      return cache.get<ArtistInfo>(cacheKey)
    }

    const { biography } = await subsonic.artistInfo(artist.id)
    const info: ArtistInfo = {
      image: artist.image || null,
      bio: biography ? { summary: biography, full: biography } : undefined,
    }

    cache.set(cacheKey, info)

    return info
  },

  /** What píxiū knows of an album: its cover and track list. */
  async fetchForAlbum(album: Album) {
    album = albumStore.syncWithVault(album)[0]
    const cacheKey = ['album.info', album.id, album.name]

    if (cache.has(cacheKey)) {
      return cache.get<AlbumInfo>(cacheKey)
    }

    const songs = await playableStore.fetchSongsForAlbum(album)
    const info: AlbumInfo = {
      cover: album.cover || null,
      tracks: songs.map(({ title, length }) => ({ title, length })),
    }

    cache.set(cacheKey, info)

    return info
  },
}
