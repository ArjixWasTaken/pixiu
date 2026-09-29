import { sampleSize } from 'lodash-es'
import { reactive } from 'vue'
import { subsonic } from '@/services/subsonic'
import { playableStore } from '@/stores/playableStore'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { recentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { isSong } from '@/utils/typeGuards'

export const overviewStore = {
  state: reactive({
    mostPlayedAlbums: [] as Album[],
    mostPlayedArtists: [] as Artist[],
    mostPlayedSongs: [] as Song[],
    randomAlbums: [] as Album[],
    randomArtists: [] as Artist[],
    recentlyAddedAlbums: [] as Album[],
    recentlyAddedArtists: [] as Artist[],
    recentlyAddedSongs: [] as Song[],
    recentlyPlayed: [] as Playable[],
    leastPlayedSongs: [] as Song[],
    randomSongs: [] as Song[],
    similarSongs: [] as Song[],
  }),

  async fetch() {
    // The album lists come from Subsonic; song and artist blocks follow later.
    const [mostPlayedAlbums, randomAlbums, recentlyAddedAlbums, randomSongs] = await Promise.all([
      subsonic.albumList('frequent', 6),
      subsonic.albumList('random', 6),
      subsonic.albumList('newest', 6),
      subsonic.randomSongs(6),
    ])

    this.state.mostPlayedAlbums = albumStore.syncWithVault(mostPlayedAlbums)
    this.state.randomAlbums = albumStore.syncWithVault(randomAlbums)
    this.state.recentlyAddedAlbums = albumStore.syncWithVault(recentlyAddedAlbums)
    this.state.randomSongs = playableStore.syncWithVault(randomSongs) as Song[]

    this.refreshPlayStats()
  },

  async refreshRandomSongs() {
    const songs = playableStore.syncWithVault(await subsonic.randomSongs(6)) as Song[]

    this.state.randomSongs = songs

    return songs
  },

  async refreshRandomAlbums() {
    const albums = albumStore.syncWithVault(await subsonic.albumList('random', 6))

    this.state.randomAlbums = albums

    return albums
  },

  async refreshRandomArtists() {
    const artists = artistStore.syncWithVault(sampleSize(await subsonic.artists(), 6))

    this.state.randomArtists = artists

    return artists
  },

  refreshPlayStats() {
    this.state.mostPlayedSongs = playableStore.getMostPlayedSongs(6)
    this.state.recentlyPlayed = recentlyPlayedStore.excerptState.playables
      .filter(playable => {
        if (isSong(playable) && playable.deleted) {
          return false
        }

        return playable.play_count > 0
      })
      .slice(0, 6)
  },
}
