import { describe, expect, it } from 'vite-plus/test'
import { customStorageEventName } from '@vueuse/core'
import { nextTick } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import { http } from '@/services/http'

const originalLocation = window.location

/** Reads what's stored, as JSON. */
const lsGet = (key: string) => JSON.parse(localStorage.getItem(key) ?? 'null')

/** Stores a value as another part of the page would: VueUse tells the service with its storage event. */
const lsSet = async (key: string, value: unknown) => {
  const newValue = value === null ? null : JSON.stringify(value)
  newValue === null ? localStorage.removeItem(key) : localStorage.setItem(key, newValue)
  window.dispatchEvent(
    new CustomEvent(customStorageEventName, { detail: { key, oldValue: null, newValue, storageArea: localStorage } }),
  )
  // VueUse holds its own writes until the next tick after taking a value in.
  await nextTick()
}

describe('authService', () => {
  const h = createHarness({
    beforeEach: () => {
      Object.defineProperty(window, 'location', {
        value: {
          ...window.location, // eslint-disable-line typescript-eslint/no-misused-spread -- intentional shallow copy for test
        },
        writable: true,
      })
    },
    afterEach: async () => {
      // @ts-ignore
      window.location = originalLocation
      await lsSet('redirect', null)
    },
  })

  it('gets the token', async () => {
    await lsSet('api-token', 'foo')
    expect(authService.getApiToken()).toBe('foo')
  })

  it.each([
    ['foo', true],
    [null, false],
  ])('checks if the token exists', async (token, exists) => {
    await lsSet('api-token', token)
    expect(authService.hasApiToken()).toBe(exists)
  })

  it('sets the token', () => {
    authService.setApiToken('foo')
    expect(lsGet('api-token')).toBe('foo')
  })

  it('destroys the token', async () => {
    await lsSet('api-token', 'foo')
    authService.destroy()
    expect(lsGet('api-token')).toBeNull()
  })

  it('signs in with a username or an email', async () => {
    const postMock = h.mock(http, 'post').mockResolvedValue({ 'audio-token': 'foo', token: 'foo' })
    h.mock(authService, 'maybeRedirect')

    await authService.login('john@doe.com', 'curry-wurst')

    expect(postMock).toHaveBeenCalledWith('auth/login', { username: 'john@doe.com', password: 'curry-wurst' })
    expect(lsGet('api-token')).toBe('foo')
  })

  it('redirects after login', async () => {
    const redirectMock = h.mock(authService, 'maybeRedirect')
    await lsSet('redirect', 'http://localhost:3000/foo/bar')

    h.mock(http, 'post').mockResolvedValue({
      'audio-token': 'foo',
      token: 'bar',
    })

    await authService.login('john@doe.com', 'curry-wurst')

    expect(redirectMock).toHaveBeenCalled()
  })

  it('sets redirect url', () => {
    authService.setRedirect('/foo/bar')
    expect(lsGet('redirect')).toBe('/foo/bar')
  })

  it('sets redirect url to the current URL', () => {
    h.mock(location, 'toString').mockReturnValue('http://localhost:3000/foo/bar')
    authService.setRedirect()
    expect(lsGet('redirect')).toBe('http://localhost:3000/foo/bar')
  })

  it('checks if redirect url exists', async () => {
    await lsSet('redirect', 'http://localhost:3000/foo/bar')
    expect(authService.hasRedirect()).toBe(true)
  })

  it('redirects to the stored URL', async () => {
    const assignMock = h.mock(location, 'assign')
    await lsSet('redirect', 'http://localhost:3000/foo/bar')

    authService.maybeRedirect()

    expect(assignMock).toHaveBeenCalledWith('http://localhost:3000/foo/bar')
    expect(lsGet('redirect')).toBeNull()
  })

  it('does not redirect if no redirect url is stored', () => {
    const assignMock = h.mock(location, 'assign')
    expect(lsGet('redirect')).toBeNull()

    authService.maybeRedirect()

    expect(assignMock).not.toHaveBeenCalled()
    expect(lsGet('redirect')).toBeNull()
  })
})
