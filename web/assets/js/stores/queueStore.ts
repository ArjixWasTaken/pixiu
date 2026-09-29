import { reactive } from 'vue'
import { differenceBy, unionBy } from 'lodash-es'
import { arrayify, moveItemsInList } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { isSong } from '@/utils/typeGuards'
import { library } from '@/services/library'
import { subsonic } from '@/services/subsonic'
import { playableStore } from '@/stores/playableStore'

export const queueStore = {
  state: reactive<{ playables: Playable[] }>({
    playables: [],
  }),

  init(savedState: QueueState) {
    // don't set this.all here, as it would trigger saving state
    this.state.playables = playableStore.syncWithVault(savedState.songs)
    this.playback = { current: savedState.current_song?.id ?? null, position: savedState.playback_position }

    if (!this.state.playables.length) {
      return
    }

    if (savedState.current_song) {
      playableStore.syncWithVault(savedState.current_song)[0].playback_state = 'Paused'
    } else {
      this.all[0].playback_state = 'Paused'
    }
  },

  get all() {
    return this.state.playables
  },

  set all(playables: Playable[]) {
    this.state.playables = playables
    playableStore.syncWithVault(playables.filter(isSong))
    this.saveState()
  },

  get first() {
    return this.all[0]
  },

  get last() {
    return this.all[this.all.length - 1]
  },

  contains(playable: Playable) {
    return this.all.includes(reactive(playable))
  },

  /**
   * Add playable(s) to the end of the current queue.
   */
  queue(playables: MaybeArray<Playable>) {
    this.unqueue(playables)
    this.all = unionBy(this.all, arrayify(playables), 'id')
  },

  queueIfNotQueued(playable: Playable, position: 'top' | 'bottom' | 'after-current' = 'after-current') {
    if (this.contains(playable)) {
      return
    }

    switch (position) {
      case 'top':
        this.queueToTop(playable)
        break
      case 'bottom':
        this.queue(playable)
        break
      case 'after-current':
        this.queueAfterCurrent(playable)
        break
    }
  },

  queueToTop(playables: MaybeArray<Playable>) {
    this.all = unionBy(arrayify(playables), this.all, 'id')
  },

  replaceQueueWith(playables: MaybeArray<Playable>) {
    this.all = arrayify(playables)
  },

  queueAfterCurrent(playables: MaybeArray<Playable>) {
    playables = arrayify(playables)

    if (!this.current || !this.all.length) {
      return this.queue(playables)
    }

    // First we unqueue the songs to make sure there are no duplicates.
    this.unqueue(playables)

    const head = this.all.splice(0, this.indexOf(this.current) + 1)
    this.all = head.concat(reactive(playables), this.all)
  },

  unqueue(playables: MaybeArray<Playable>) {
    playables = arrayify(playables)
    playables.forEach(song => (song.playback_state = 'Stopped'))
    this.all = differenceBy(this.all, playables, 'id')
  },

  /**
   * Move some songs to after a target.
   */
  move(playables: MaybeArray<Playable>, target: Playable, placement: Placement) {
    this.state.playables = moveItemsInList(this.state.playables, playables, target, placement)
    this.saveState()
  },

  clear() {
    this.all = []
  },

  /**
   * Clear the queue without saving the state.
   */
  clearSilently() {
    this.state.playables = []
  },

  indexOf(playable: Playable) {
    return this.all.indexOf(reactive(playable))
  },

  get next() {
    if (!this.current) {
      return this.first
    }

    const index = this.indexOf(this.current) + 1

    return index >= this.all.length ? undefined : this.all[index]
  },

  get previous() {
    if (!this.current) {
      return this.last
    }

    const index = this.indexOf(this.current) - 1

    return index < 0 ? undefined : this.all[index]
  },

  get current() {
    // Search the queue first (reactive array — triggers Vue computed re-evaluation).
    // Fall back to the vault for songs removed from the queue (e.g. after replaceQueueWith).
    return this.all.find(({ playback_state }) => playback_state !== 'Stopped') || playableStore.findPlaying()
  },

  async fetchRandom(limit = 500) {
    this.all = await subsonic.randomSongs(limit)
    return this.all
  },

  async fetchInOrder(sortField: PlayableListSortField, order: SortOrder, limit = 500) {
    this.all = (await library.songs({ sort: sortField, order, limit })).items
    return this.all
  },

  /** What is playing and where, saved with the queue. */
  playback: { current: null as Playable['id'] | null, position: 0 },

  saveState() {
    subsonic
      .savePlayQueue(
        this.state.playables.map(({ id }) => id),
        this.playback.current,
        this.playback.position,
      )
      .catch(error => logger.error(error))
  },

  /** Saves the queue with the playing song and its position (seconds). */
  savePlaybackStatus(playable: Playable, position: number) {
    if (this.playback.current === playable.id && this.playback.position === position) {
      return
    }

    this.playback = { current: playable.id, position }
    this.saveState()
  },
}
