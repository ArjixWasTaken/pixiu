import { queryClient } from '@/services/queryClient'
import { subsonic } from '@/services/subsonic'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { usePlayableStore } from '@/stores/playableStore'

export const encyclopediaService = {
  /** What píxiū knows of an artist: their image and biography. */
  async fetchForArtist(artist: Artist) {
    artist = useArtistStore().syncWithVault(artist)[0]

    return queryClient.fetchQuery({
      queryKey: ['artist', artist.id, 'info'],
      queryFn: async (): Promise<ArtistInfo> => {
        const { biography } = await subsonic.artistInfo(artist.id)

        return {
          image: artist.image || null,
          bio: biography ? { summary: biography, full: biography } : undefined,
        }
      },
    })
  },

  /** What píxiū knows of an album: its cover and track list. */
  async fetchForAlbum(album: Album) {
    album = useAlbumStore().syncWithVault(album)[0]

    return queryClient.fetchQuery({
      // Under the album's own key: editing it makes this stale too.
      queryKey: ['album', album.id, 'info'],
      queryFn: async (): Promise<AlbumInfo> => {
        const songs = await usePlayableStore().fetchSongsForAlbum(album)

        return {
          cover: album.cover || null,
          tracks: songs.map(({ title, length }) => ({ title, length })),
        }
      },
    })
  },
}
