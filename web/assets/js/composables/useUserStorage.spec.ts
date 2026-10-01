import { describe, expect, it } from 'vite-plus/test'
import { effectScope } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useAppStorage, useUserStorage } from '@/composables/useUserStorage'

describe('useUserStorage', () => {
  const h = createHarness({
    afterEach: () => localStorage.clear(),
  })

  const run = <T>(fn: () => T) => effectScope().run(fn)!

  it('keeps a value apart for each user', () => {
    const alice = h.factory('user').make()
    h.actingAsUser(alice as CurrentUser)
    run(() => useUserStorage('sidebar-collapsed', false)).value = true

    expect(localStorage.getItem(`${alice.id}::sidebar-collapsed`)).toBe('true')

    h.actingAsUser(h.factory('user').make() as CurrentUser)
    expect(run(() => useUserStorage('sidebar-collapsed', false)).value).toBe(false)
  })

  it('reads what was saved before, as JSON', () => {
    localStorage.setItem('api-token', JSON.stringify('foo'))

    expect(run(() => useAppStorage<string | null>('api-token', null)).value).toBe('foo')
  })

  it('writes nothing for a default, and at once for a change', () => {
    const token = run(() => useAppStorage<string | null>('api-token', null))
    expect(localStorage.getItem('api-token')).toBeNull()

    token.value = 'bar'
    expect(localStorage.getItem('api-token')).toBe('"bar"')
  })
})
