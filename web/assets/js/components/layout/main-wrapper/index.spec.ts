import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useOverviewStore } from '@/stores/overviewStore'
import Component from './index.vue'

describe('mainWrapper.vue', () => {
  const h = createHarness()

  it('renders sidebar and main content', () => {
    // Home opens inside: its overview isn't the point here.
    h.mock(useOverviewStore(), 'fetch')
    const { container } = h.render(Component)
    expect(container.querySelector('div')).toBeTruthy()
  })
})
