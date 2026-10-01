import { defineStore } from 'pinia'
import { computed, reactive } from 'vue'
import { differenceBy, unionBy } from 'lodash-es'
import { arrayify, moveItemsInList } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { usePlayableStore } from '@/stores/playableStore'

/** What plays next, and where playback is; saved on the server (Subsonic's play queue). */
export const useQueueStore = defineStore('queue', () => {
  const state = reactive<{ playables: Playable[] }>({
    playables: [],
  })

  /** What is playing and where, saved with the queue. */
  let playback = { current: null as Playable['id'] | null, position: 0 }

  const saveState = () => {
    subsonic
      .savePlayQueue(
        state.playables.map(({ id }) => id),
        playback.current,
        playback.position,
      )
      .catch(error => logger.error(error))
  }

  /** The queue; setting it saves it. */
  const all = computed<Playable[]>({
    get: () => state.playables,
    set: playables => {
      state.playables = playables
      usePlayableStore().syncWithVault(playables)
      saveState()
    },
  })

  const first = computed(() => all.value[0])
  const last = computed(() => all.value[all.value.length - 1])

  const indexOf = (playable: Playable) => all.value.indexOf(reactive(playable))

  const current = computed(
    () =>
      // Search the queue first (reactive array — triggers Vue computed re-evaluation).
      // Fall back to the vault for songs removed from the queue (e.g. after replaceQueueWith).
      all.value.find(({ playback_state }) => playback_state !== 'Stopped') || usePlayableStore().findPlaying(),
  )

  const next = computed(() => {
    if (!current.value) {
      return first.value
    }

    const index = indexOf(current.value) + 1

    return index >= all.value.length ? undefined : all.value[index]
  })

  const previous = computed(() => {
    if (!current.value) {
      return last.value
    }

    const index = indexOf(current.value) - 1

    return index < 0 ? undefined : all.value[index]
  })

  const init = (savedState: QueueState) => {
    const playableStore = usePlayableStore()

    // Not through `all`: that would save the state just loaded.
    state.playables = playableStore.syncWithVault(savedState.songs)
    playback = { current: savedState.current_song?.id ?? null, position: savedState.playback_position }

    if (!state.playables.length) {
      return
    }

    if (savedState.current_song) {
      playableStore.syncWithVault(savedState.current_song)[0].playback_state = 'Paused'
    } else {
      all.value[0].playback_state = 'Paused'
    }
  }

  const contains = (playable: Playable) => all.value.includes(reactive(playable))

  const unqueue = (playables: MaybeArray<Playable>) => {
    playables = arrayify(playables)
    playables.forEach(song => (song.playback_state = 'Stopped'))
    all.value = differenceBy(all.value, playables, 'id')
  }

  /**
   * Add playable(s) to the end of the current queue.
   */
  const queue = (playables: MaybeArray<Playable>) => {
    unqueue(playables)
    all.value = unionBy(all.value, arrayify(playables), 'id')
  }

  const queueToTop = (playables: MaybeArray<Playable>) => {
    all.value = unionBy(arrayify(playables), all.value, 'id')
  }

  const replaceQueueWith = (playables: MaybeArray<Playable>) => {
    all.value = arrayify(playables)
  }

  const queueAfterCurrent = (playables: MaybeArray<Playable>) => {
    playables = arrayify(playables)

    if (!current.value || !all.value.length) {
      return queue(playables)
    }

    // First we unqueue the songs to make sure there are no duplicates.
    unqueue(playables)

    const rest = [...all.value]
    const head = rest.splice(0, indexOf(current.value) + 1)
    all.value = head.concat(reactive(playables), rest)
  }

  const queueIfNotQueued = (playable: Playable, position: 'top' | 'bottom' | 'after-current' = 'after-current') => {
    if (contains(playable)) {
      return
    }

    switch (position) {
      case 'top':
        queueToTop(playable)
        break
      case 'bottom':
        queue(playable)
        break
      case 'after-current':
        queueAfterCurrent(playable)
        break
    }
  }

  /**
   * Move some songs to after a target.
   */
  const move = (playables: MaybeArray<Playable>, target: Playable, placement: Placement) => {
    state.playables = moveItemsInList(state.playables, playables, target, placement)
    saveState()
  }

  const clear = () => {
    all.value = []
  }

  /**
   * Clear the queue without saving the state.
   */
  const clearSilently = () => {
    state.playables = []
  }

  const fetchRandom = async (limit = 500) => {
    all.value = await subsonic.randomSongs(limit)
    return all.value
  }

  const fetchInOrder = async (sortField: PlayableListSortField, order: SortOrder, limit = 500) => {
    all.value = (await library.songs({ sort: sortField, order, limit })).items
    return all.value
  }

  /** Saves the queue with the playing song and its position (seconds). */
  const savePlaybackStatus = (playable: Playable, position: number) => {
    if (playback.current === playable.id && playback.position === position) {
      return
    }

    playback = { current: playable.id, position }
    saveState()
  }

  return {
    state,
    all,
    first,
    last,
    current,
    next,
    previous,
    init,
    contains,
    queue,
    queueIfNotQueued,
    queueToTop,
    replaceQueueWith,
    queueAfterCurrent,
    unqueue,
    move,
    clear,
    clearSilently,
    indexOf,
    fetchRandom,
    fetchInOrder,
    saveState,
    savePlaybackStatus,
  }
})
