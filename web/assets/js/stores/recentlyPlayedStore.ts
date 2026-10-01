import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { library } from '@/services/library'
import { usePlayableStore } from '@/stores/playableStore'

const EXCERPT_COUNT = 6

export const useRecentlyPlayedStore = defineStore('recentlyPlayed', () => {
  const excerptState = reactive({
    playables: [] as Playable[],
  })

  const state = reactive({
    playables: [] as Playable[],
  })

  const fetch = async () => {
    state.playables = usePlayableStore().syncWithVault(await library.recentlyPlayed(100))
    return state.playables
  }

  const add = async (playable: Playable) => {
    if (!state.playables.length) {
      await fetch()
    }

    ;[state, excerptState].forEach(each => {
      each.playables = each.playables.filter(s => s.id !== playable.id)
      each.playables.unshift(playable)
    })

    excerptState.playables.splice(EXCERPT_COUNT)
  }

  return { excerptState, state, fetch, add }
})
