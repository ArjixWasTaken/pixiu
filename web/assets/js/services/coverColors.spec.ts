import { describe, expect, it, vi } from 'vite-plus/test'
import { argbFromHex } from '@material/material-color-utilities'
import { createHarness } from '@/__tests__/TestHarness'
import { paintRoot, schemeFrom, sourceColorOf } from '@/services/coverColors'

describe('coverColors', () => {
  createHarness({
    afterEach: () => {
      paintRoot(null)
      vi.unstubAllGlobals()
    },
  })

  /** A cover of one color, as the canvas reads it back. */
  const stubCover = (rgb: [number, number, number]) => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, blob: async () => new Blob(['x']) })
    const pixels = new Uint8ClampedArray(112 * 112 * 4)

    for (let i = 0; i < pixels.length; i += 4) {
      pixels.set([...rgb, 255], i)
    }

    vi.stubGlobal('fetch', fetchMock)
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue({ close: vi.fn() }))
    vi.stubGlobal(
      'OffscreenCanvas',
      class {
        getContext = () => ({ drawImage: vi.fn(), getImageData: () => ({ data: pixels }) })
      },
    )

    return fetchMock
  }

  it('builds every role from a source color, for either mode', () => {
    const dark = schemeFrom(argbFromHex('#3366cc'), true)
    const light = schemeFrom(argbFromHex('#3366cc'), false)

    expect(Object.keys(dark)).toHaveLength(49)
    expect(dark['--schemes-surface-container-highest']).toMatch(/^#[0-9a-f]{6}$/)
    expect(dark['--schemes-on-primary-fixed-variant']).toMatch(/^#[0-9a-f]{6}$/)
    expect(dark['--schemes-surface']).not.toBe(light['--schemes-surface'])
  })

  it('reads a small copy of a cover, once', async () => {
    const fetchMock = stubCover([200, 40, 40])

    const url = 'http://localhost:3000/rest/getCoverArt?id=al-1'
    const first = await sourceColorOf(url)
    const second = await sourceColorOf(url)

    expect(first).toBe(second)
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock).toHaveBeenCalledWith('http://localhost:3000/rest/getCoverArt?id=al-1&size=112')
  })

  it('gives up on a cover it cannot read', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false }))

    expect(await sourceColorOf('http://localhost:3000/rest/getCoverArt?id=al-gone')).toBeNull()
  })

  it('paints the roots and takes them off again', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }))
    const colors = schemeFrom(argbFromHex('#3366cc'), true)

    paintRoot(colors)
    expect(document.documentElement.style.getPropertyValue('--schemes-primary')).toBe(colors['--schemes-primary'])

    paintRoot(null)
    expect(document.documentElement.style.getPropertyValue('--schemes-primary')).toBe('')
  })

  it('fades into new colors unless motion is reduced', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: false }))
    const startViewTransition = vi.fn((update: () => void) => update())
    Object.assign(document, { startViewTransition })

    paintRoot(schemeFrom(argbFromHex('#33cc66'), true))

    expect(startViewTransition).toHaveBeenCalled()
    delete (document as any).startViewTransition
  })
})
