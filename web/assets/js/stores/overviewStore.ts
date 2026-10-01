import { defineStore } from 'pinia'
import { sampleSize } from 'lodash-es'
import { reactive } from 'vue'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { usePlayableStore } from '@/stores/playableStore'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'

/** What Home shows. */
export const useOverviewStore = defineStore('overview', () => {
  const state = reactive({
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
  })

  const refreshPlayStats = () => {
    state.mostPlayedSongs = usePlayableStore().getMostPlayedSongs(6)
    state.recentlyPlayed = useRecentlyPlayedStore()
      .excerptState.playables.filter(playable => !playable.deleted && playable.play_count > 0)
      .slice(0, 6)
  }

  const refreshRandomSongs = async () => {
    const songs = usePlayableStore().syncWithVault(await subsonic.randomSongs(6)) as Song[]

    state.randomSongs = songs

    return songs
  }

  const refreshRandomAlbums = async () => {
    const albums = useAlbumStore().syncWithVault(await subsonic.albumList('random', 6))

    state.randomAlbums = albums

    return albums
  }

  const refreshRandomArtists = async () => {
    const artists = useArtistStore().syncWithVault(sampleSize(await subsonic.artists(), 6))

    state.randomArtists = artists

    return artists
  }

  const fetch = async () => {
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
      refreshRandomArtists(),
    ])

    const albumStore = useAlbumStore()
    const playableStore = usePlayableStore()

    state.mostPlayedAlbums = albumStore.syncWithVault(mostPlayedAlbums)
    state.randomAlbums = albumStore.syncWithVault(randomAlbums)
    state.recentlyAddedAlbums = albumStore.syncWithVault(recentlyAddedAlbums)
    state.randomSongs = playableStore.syncWithVault(randomSongs) as Song[]
    // Most played songs come from the vault in refreshPlayStats.
    playableStore.syncWithVault(mostPlayedSongs.items)
    state.leastPlayedSongs = playableStore.syncWithVault(leastPlayedSongs.items) as Song[]
    state.recentlyAddedSongs = playableStore.syncWithVault(recentlyAddedSongs.items) as Song[]
    state.recentlyAddedArtists = useArtistStore().syncWithVault(recentlyAddedArtists.items)
    useRecentlyPlayedStore().excerptState.playables = playableStore.syncWithVault(recentlyPlayed)

    refreshPlayStats()
  }

  return { state, fetch, refreshRandomSongs, refreshRandomAlbums, refreshRandomArtists, refreshPlayStats }
})
