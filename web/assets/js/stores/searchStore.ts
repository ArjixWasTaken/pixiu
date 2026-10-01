import { reactive } from 'vue'
import { subsonic } from '@/services/subsonic'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'

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

export const searchStore = {
  state: reactive({
    excerpt: {
      playables: [],
      albums: [],
      artists: [],
    } as ExcerptState,
    playables: [] as Playable[],
  }),

  async excerptSearch(q: string) {
    const result = await subsonic.search(q, 6)

    this.state.excerpt.playables = playableStore.syncWithVault(result.songs)
    this.state.excerpt.albums = albumStore.syncWithVault(result.albums)
    this.state.excerpt.artists = artistStore.syncWithVault(result.artists)
  },

  async playableSearch(q: string) {
    this.state.playables = playableStore.syncWithVault((await subsonic.search(q, 500)).songs)
  },

  resetPlayableResultState() {
    this.state.playables = []
  },
}
