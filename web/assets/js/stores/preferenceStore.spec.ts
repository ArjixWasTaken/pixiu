import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { http } from '@/services/http'
import { usePreferenceStore } from '@/stores/preferenceStore'
describe('preferenceStore', () => {
  const h = createHarness({
    beforeEach: () => usePreferenceStore().init(),
  })

  it('does not trigger a request if the value is the same', () => {
    const mock = h.mock(http, 'patch')
    usePreferenceStore().set('volume', usePreferenceStore().volume)
    expect(mock).not.toHaveBeenCalled()
  })
})
