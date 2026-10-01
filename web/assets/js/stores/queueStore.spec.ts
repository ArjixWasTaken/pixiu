import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { usePlayableStore } from '@/stores/playableStore'
import { useQueueStore } from '@/stores/queueStore'
describe('queueStore', () => {
  let songs: Song[] = []

  const h = createHarness({
    beforeEach: () => {
      usePlayableStore().vault.clear()
      songs = usePlayableStore().syncWithVault(h.factory('song').make(3)) as Song[]
      useQueueStore().state.playables = reactive(songs)
    },
  })

  it('returns all queued songs', () => expect(useQueueStore().all).toEqual(songs))

  it('returns the first queued song', () => expect(useQueueStore().first).toEqual(songs[0]))

  it('returns the last queued song', () => expect(useQueueStore().last).toEqual(songs[2]))

  it.each<[PlaybackState]>([['Playing'], ['Paused']])('identifies the current song by %s state', state => {
    useQueueStore().state.playables[1].playback_state = state
    expect(useQueueStore().current).toEqual(useQueueStore().state.playables[1])
  })

  it('gets the next song in queue', () => {
    useQueueStore().state.playables[1].playback_state = 'Playing'
    expect(useQueueStore().next).toEqual(useQueueStore().state.playables[2])
  })

  it('returns undefined as next song if at end of queue', () => {
    useQueueStore().state.playables[2].playback_state = 'Playing'
    expect(useQueueStore().next).toBeUndefined()
  })

  it('gets the previous song in queue', () => {
    useQueueStore().state.playables[1].playback_state = 'Playing'
    expect(useQueueStore().previous).toEqual(useQueueStore().state.playables[0])
  })

  it('returns undefined as previous song if at beginning of queue', () => {
    useQueueStore().state.playables[0].playback_state = 'Playing'
    expect(useQueueStore().previous).toBeUndefined()
  })
})
