import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { huntingService } from '@/services/huntingService'
import Component from './OrphansScreen.vue'

const INTRO = /Nothing keeps these any more/

describe('orphansScreen.vue', () => {
  const h = createHarness()

  it('explains what orphans are while there are some', async () => {
    const song = h.factory('song').make({ title: 'Left behind' })
    h.mock(huntingService, 'orphans').mockResolvedValue({
      orphans: [{ song, reason: 'Left “Road trip”', released_at: null, size: 1024 }],
      totalSize: 1024,
    })
    h.render(Component)

    await screen.findByText('Left behind')
    screen.getByText(INTRO)
  })

  it('leaves the explanation out when there are none', async () => {
    const orphans = h.mock(huntingService, 'orphans').mockResolvedValue({ orphans: [], totalSize: 0 })
    h.render(Component)

    await h.tick(2)
    expect(orphans).toHaveBeenCalled()
    expect(screen.queryByText(INTRO)).toBeNull()
  })
})
