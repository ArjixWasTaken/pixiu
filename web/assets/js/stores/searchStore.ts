import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { subsonic } from '@/services/subsonic'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { usePlayableStore } from '@/stores/playableStore'

export interface ExcerptState {
  playables: Playable[]
  albums: Album[]
  artists: Artist[]
}

export interface ExcerptSearchResult {
  songs: Playable[] // backward compatibility
  albums: Album[]
  artists: Artist[]
}

export const useSearchStore = defineStore('search', () => {
  const state = reactive({
    playables: [] as Playable[],
  })

  /** A few of each kind matching `q`, for the search screen (which keeps them by `q`, see there). */
  const excerptSearch = async (q: string): Promise<ExcerptState> => {
    const result = await subsonic.search(q, 6)

    return {
      playables: usePlayableStore().syncWithVault(result.songs),
      albums: useAlbumStore().syncWithVault(result.albums),
      artists: useArtistStore().syncWithVault(result.artists),
    }
  }

  const playableSearch = async (q: string) => {
    state.playables = usePlayableStore().syncWithVault((await subsonic.search(q, 500)).songs)
  }

  const resetPlayableResultState = () => {
    state.playables = []
  }

  return { state, excerptSearch, playableSearch, resetPlayableResultState }
})
