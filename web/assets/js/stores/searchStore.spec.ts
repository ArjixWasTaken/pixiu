import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useSearchStore } from '@/stores/searchStore'

describe('searchStore', () => {
  // Each spec has stores of its own, fresh.
  const h = createHarness()

  it('resets the song result state', () => {
    useSearchStore().state.playables = h.factory('song').make(3)
    useSearchStore().resetPlayableResultState()
    expect(useSearchStore().state.playables).toEqual([])
  })
})
