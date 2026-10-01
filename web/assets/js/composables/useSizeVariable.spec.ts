import { describe, expect, it } from 'vite-plus/test'
import { effectScope } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useSizeVariable } from '@/composables/useSizeVariable'

describe('useSizeVariable', () => {
  createHarness({
    afterEach: () => document.documentElement.style.removeProperty('--m3-row-height'),
  })

  const read = () => effectScope().run(() => useSizeVariable('--m3-row-height', 72))!.value

  it('reads the size the stylesheet sets', () => {
    document.documentElement.style.setProperty('--m3-row-height', '48px')

    expect(read()).toBe(48)
  })

  it('falls back without one', () => {
    expect(read()).toBe(72)
  })
})
