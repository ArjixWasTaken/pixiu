import { afterEach, describe, expect, it } from 'vite-plus/test'
import { toClientPath } from './clientUrl'

describe('toClientPath', () => {
  afterEach(() => (window.KOEL.base_url = '/'))

  it('keeps a path at the root as it is', () => {
    expect(toClientPath('/albums/al-1')).toBe('/albums/al-1')
    expect(toClientPath('albums')).toBe('/albums')
  })

  it('takes off the base the app is served under', () => {
    window.KOEL.base_url = '/music/'
    expect(toClientPath('/music/albums?sort=name#x')).toBe('/albums?sort=name#x')
  })
})
