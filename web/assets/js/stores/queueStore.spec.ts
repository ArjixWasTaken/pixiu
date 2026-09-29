import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playableStore } from '@/stores/playableStore'
import { queueStore } from '@/stores/queueStore'

describe('queueStore', () => {
  let songs: Song[] = []

  const h = createHarness({
    beforeEach: () => {
      playableStore.vault.clear()
      songs = playableStore.syncWithVault(h.factory('song').make(3)) as Song[]
      queueStore.state.playables = reactive(songs)
    },
  })

  it('returns all queued songs', () => expect(queueStore.all).toEqual(songs))

  it('returns the first queued song', () => expect(queueStore.first).toEqual(songs[0]))

  it('returns the last queued song', () => expect(queueStore.last).toEqual(songs[2]))

  it.each<[PlaybackState]>([['Playing'], ['Paused']])('identifies the current song by %s state', state => {
    queueStore.state.playables[1].playback_state = state
    expect(queueStore.current).toEqual(queueStore.state.playables[1])
  })

  it('gets the next song in queue', () => {
    queueStore.state.playables[1].playback_state = 'Playing'
    expect(queueStore.next).toEqual(queueStore.state.playables[2])
  })

  it('returns undefined as next song if at end of queue', () => {
    queueStore.state.playables[2].playback_state = 'Playing'
    expect(queueStore.next).toBeUndefined()
  })

  it('gets the previous song in queue', () => {
    queueStore.state.playables[1].playback_state = 'Playing'
    expect(queueStore.previous).toEqual(queueStore.state.playables[0])
  })

  it('returns undefined as previous song if at beginning of queue', () => {
    queueStore.state.playables[0].playback_state = 'Playing'
    expect(queueStore.previous).toBeUndefined()
  })
})
