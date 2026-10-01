import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useRouter } from '@/composables/useRouter'
import Router, { createAppRouter, notFound } from '@/router'

describe('router', () => {
  const h = createHarness()

  const at = () => h.router.currentRoute.value.fullPath

  describe('routes', () => {
    it('makes the address of a named route', () => {
      expect(Router.url('home')).toBe('/home')
      expect(Router.url('albums.show', { id: 'al-1' })).toBe('/albums/al-1')
    })

    it('refuses an unknown route, or one missing a parameter', () => {
      expect(() => Router.url('nope' as never)).toThrow()
      expect(() => Router.url('albums.show')).toThrow()
    })

    it('opens Home at the root, keeping what the server sent along', async () => {
      await h.visit('/?sso_error=busy')
      expect(at()).toBe('/home?sso_error=busy')
    })

    it.each([
      ['/albums/al-1/information', '/albums/al-1#information'],
      ['/artists/ar-1/albums', '/artists/ar-1#albums'],
      ['/settings?tab=users', '/settings#admin-users'],
      ['/settings?tab=account&linked=1', '/settings?linked=1#account'],
      ['/profile', '/settings#preferences'],
    ])('takes an old link (%s) to its tab in the hash', async (from, to) => {
      await h.visit(from)
      expect(at()).toBe(to)
    })

    it('opens a shared song in the queue', async () => {
      await h.visit('/songs/tr-7')

      expect(at()).toBe('/queue?song=tr-7')
      expect(useRouter().getRouteParam('song')).toBe('tr-7')
    })

    it('shows the 404 screen for an address it does not know, at that address', async () => {
      await h.visit('/nowhere/at/all')

      expect(at()).toBe('/nowhere/at/all')
      expect(useRouter().getCurrentScreen()).toBe('404')
    })
  })

  describe('guards', () => {
    it('show the 404 screen where the user may not go, once guarding', async () => {
      const { startGuarding, isCurrentScreen } = useRouter()

      await h.visit('/upload')
      expect(isCurrentScreen('Upload')).toBe(true)

      startGuarding()
      expect(isCurrentScreen('404')).toBe(true)

      await h.visit('/home')
      expect(isCurrentScreen('Home')).toBe(true)
    })

    it('let through who may', async () => {
      h.actingAsAdmin()
      useRouter().startGuarding()

      await h.visit('/upload')
      expect(useRouter().isCurrentScreen('Upload')).toBe(true)
    })
  })

  describe('route changes', () => {
    it('tells the screens, but not of a tab moving in the hash', async () => {
      const handler = vi.fn()
      useRouter().onRouteChanged(handler)

      await h.visit('/settings')
      expect(handler).toHaveBeenCalledWith(expect.objectContaining({ screen: 'Settings' }), expect.anything())

      handler.mockClear()
      await h.router.replace({ path: '/settings', hash: '#admin-users' })
      expect(handler).not.toHaveBeenCalled()
    })

    it('clear a 404 a screen asked for', async () => {
      const { triggerNotFound, isCurrentScreen } = useRouter()

      triggerNotFound()
      expect(isCurrentScreen('404')).toBe(true)

      await h.visit('/albums')
      expect(notFound.value).toBe(false)
      expect(isCurrentScreen('Albums')).toBe(true)
    })
  })

  describe('links', () => {
    /** Whether the app took the click. The browser's own navigation is cancelled either way: jsdom has none. */
    const click = (href: string, init: MouseEventInit = {}) => {
      const link = Object.assign(document.createElement('a'), { href })
      document.body.appendChild(link)

      let taken = false
      const after = (event: Event) => {
        taken = event.defaultPrevented
        event.preventDefault()
      }

      // Listening after the router's own listener, which was there first.
      window.addEventListener('click', after)
      link.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, button: 0, ...init }))
      window.removeEventListener('click', after)

      return taken
    }

    it('keep a click on a link to a screen inside the app', async () => {
      expect(click('/albums/al-1#other-albums')).toBe(true)
      await vi.waitFor(() => expect(at()).toBe('/albums/al-1#other-albums'))
    })

    it('leave anything that is not a screen to the browser', async () => {
      expect(click('/rest/stream?id=tr-1')).toBe(false)
      expect(click('https://musicbrainz.org/release/x')).toBe(false)
    })

    it('leave a click with a modifier key to the browser', async () => {
      expect(click('/albums', { ctrlKey: true })).toBe(false)
    })
  })

  it('turns an old hash address into a plain path', () => {
    history.replaceState(null, '', '/#/albums/al-1')
    createAppRouter()

    expect(`${location.pathname}${location.hash}`).toBe('/albums/al-1')
    history.replaceState(null, '', '/')
  })
})
