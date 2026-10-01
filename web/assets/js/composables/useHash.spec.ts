import { describe, expect, it, vi } from 'vite-plus/test'
import { effectScope, nextTick } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { moveTabToHash, useHash, useHashTab } from '@/composables/useHash'

describe('useHash', () => {
  createHarness({
    afterEach: () => history.replaceState(null, '', '/'),
  })

  const run = <T>(fn: () => T) => effectScope().run(fn)!

  it('reads the hash, without the #', () => {
    history.replaceState(null, '', '/settings#admin-users')

    expect(run(useHash).value).toBe('admin-users')
  })

  it('replaces the URL when set, without a navigation for the router', async () => {
    history.replaceState(null, '', '/settings?linked=1')
    const length = history.length
    const popstate = vi.fn()
    window.addEventListener('popstate', popstate)

    const hash = run(useHash)
    hash.value = 'account'
    await nextTick()

    expect(`${location.pathname}${location.search}${location.hash}`).toBe('/settings?linked=1#account')
    expect(history.length).toBe(length)
    expect(popstate).not.toHaveBeenCalled()
    window.removeEventListener('popstate', popstate)
  })

  it('follows the hash when it changes elsewhere', () => {
    const hash = run(useHash)

    history.replaceState(null, '', '/settings#preferences')
    window.dispatchEvent(new HashChangeEvent('hashchange'))

    expect(hash.value).toBe('preferences')
  })

  it('keeps a tab to the ones there are', () => {
    history.replaceState(null, '', '/albums/al-1#nonsense')

    expect(run(() => useHashTab(['songs', 'information'] as const, 'songs')).value).toBe('songs')
  })

  it('moves a tab from the path or the query to the hash', () => {
    history.replaceState(null, '', '/albums/al-1/information')
    moveTabToHash('information')
    expect(`${location.pathname}${location.hash}`).toBe('/albums/al-1#information')

    history.replaceState(null, '', '/settings?tab=users&linked=1')
    moveTabToHash('users', 'admin-users')
    expect(`${location.pathname}${location.search}${location.hash}`).toBe('/settings?linked=1#admin-users')
  })
})
