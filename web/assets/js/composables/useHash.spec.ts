import { describe, expect, it, vi } from 'vite-plus/test'
import { effectScope } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useHash, useHashTab } from '@/composables/useHash'
import { useRouter } from '@/composables/useRouter'

describe('useHash', () => {
  const h = createHarness()

  const run = <T>(fn: () => T) => effectScope().run(fn)!
  const at = () => h.router.currentRoute.value.fullPath

  it('reads the hash, without the #', async () => {
    await h.visit('/settings#admin-users')

    expect(run(useHash).value).toBe('admin-users')
  })

  it('replaces the address when set, without telling the screens of a new route', async () => {
    await h.visit('/settings?linked=1')
    const changed = vi.fn()
    useRouter().onRouteChanged(changed)

    const hash = run(useHash)
    hash.value = 'account'

    await vi.waitFor(() => expect(at()).toBe('/settings?linked=1#account'))
    expect(changed).not.toHaveBeenCalled()
  })

  it('follows the hash when it changes elsewhere', async () => {
    await h.visit('/settings')
    const hash = run(useHash)

    await h.visit('/settings#preferences')

    expect(hash.value).toBe('preferences')
  })

  it('keeps a tab to the ones there are', async () => {
    await h.visit('/albums/al-1#nonsense')

    expect(run(() => useHashTab(['songs', 'information'] as const, 'songs')).value).toBe('songs')
  })
})
