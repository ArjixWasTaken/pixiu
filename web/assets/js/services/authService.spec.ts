import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { authService } from '@/services/authService'
import { http } from '@/services/http'
import { useLocalStorage } from '@/composables/useLocalStorage'

const originalLocation = window.location

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
    afterEach: () => {
      // @ts-ignore
      window.location = originalLocation
      useLocalStorage(false).remove('redirect')
    },
  })

  const { get: lsGet, set: lsSet } = useLocalStorage(false)

  it('gets the token', () => {
    lsSet('api-token', 'foo')
    expect(authService.getApiToken()).toBe('foo')
  })

  it.each([
    ['foo', true],
    [null, false],
  ])('checks if the token exists', (token, exists) => {
    lsSet('api-token', token)
    expect(authService.hasApiToken()).toBe(exists)
  })

  it('sets the token', () => {
    authService.setApiToken('foo')
    expect(lsGet('api-token')).toBe('foo')
  })

  it('destroys the token', () => {
    lsSet('api-token', 'foo')
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
    lsSet('redirect', 'http://localhost:3000/foo/bar')

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

  it('checks if redirect url exists', () => {
    lsSet('redirect', 'http://localhost:3000/foo/bar')
    expect(authService.hasRedirect()).toBe(true)
  })

  it('redirects to the stored URL', () => {
    const assignMock = h.mock(location, 'assign')
    lsSet('redirect', 'http://localhost:3000/foo/bar')

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
