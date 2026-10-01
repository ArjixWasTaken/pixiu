import { sampleSize } from 'lodash-es'
import { reactive } from 'vue'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { playableStore } from '@/stores/playableStore'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { recentlyPlayedStore } from '@/stores/recentlyPlayedStore'

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
    const [
      mostPlayedAlbums,
      randomAlbums,
      recentlyAddedAlbums,
      randomSongs,
      mostPlayedSongs,
      leastPlayedSongs,
      recentlyAddedSongs,
      recentlyAddedArtists,
      recentlyPlayed,
    ] = await Promise.all([
      subsonic.albumList('frequent', 6),
      subsonic.albumList('random', 6),
      subsonic.albumList('newest', 6),
      subsonic.randomSongs(6),
      library.songs({ sort: 'play_count', order: 'desc', limit: 6 }),
      library.songs({ sort: 'play_count', order: 'asc', limit: 6 }),
      library.songs({ sort: 'created_at', order: 'desc', limit: 6 }),
      library.artists({ sort: 'created_at', order: 'desc', limit: 6 }),
      library.recentlyPlayed(6),
      this.refreshRandomArtists(),
    ])

    this.state.mostPlayedAlbums = albumStore.syncWithVault(mostPlayedAlbums)
    this.state.randomAlbums = albumStore.syncWithVault(randomAlbums)
    this.state.recentlyAddedAlbums = albumStore.syncWithVault(recentlyAddedAlbums)
    this.state.randomSongs = playableStore.syncWithVault(randomSongs) as Song[]
    // Most played songs come from the vault in refreshPlayStats.
    playableStore.syncWithVault(mostPlayedSongs.items)
    this.state.leastPlayedSongs = playableStore.syncWithVault(leastPlayedSongs.items) as Song[]
    this.state.recentlyAddedSongs = playableStore.syncWithVault(recentlyAddedSongs.items) as Song[]
    this.state.recentlyAddedArtists = artistStore.syncWithVault(recentlyAddedArtists.items)
    recentlyPlayedStore.excerptState.playables = playableStore.syncWithVault(recentlyPlayed)

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
        if (playable.deleted) {
          return false
        }

        return playable.play_count > 0
      })
      .slice(0, 6)
  },
}
