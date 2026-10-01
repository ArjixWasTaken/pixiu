import { describe, expect, it, vi } from 'vite-plus/test'
import { defineComponent, nextTick, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useQueueStore } from '@/stores/queueStore'
import { useThemeStore } from '@/stores/themeStore'
import { useCoverTheme, useCoverTint } from '@/composables/useCoverTheme'

const paintRoot = vi.fn()
const sourceColorOf = vi.fn()

vi.mock('@/services/coverColors', () => ({
  paintRoot: (colors: unknown) => paintRoot(colors),
  sourceColorOf: (url: string) => sourceColorOf(url),
  schemeFrom: (source: number, dark: boolean) => ({ '--schemes-primary': `${source}-${dark ? 'dark' : 'light'}` }),
}))

describe('useCoverTheme', () => {
  const h = createHarness({
    beforeEach: () => {
      paintRoot.mockClear()
      sourceColorOf.mockReset()
    },
    afterEach: () => {
      usePreferenceStore().state.theme = 'cover'
    },
  })

  const settle = async () => {
    await nextTick()
    await new Promise(resolve => setTimeout(resolve))
  }

  const mount = (setup: () => void) => h.render(defineComponent({ setup, render: () => null }))

  const playing = (cover: string) => {
    const song = h.factory('song').make({ album_cover: cover, playback_state: 'Playing' })
    useQueueStore().state.playables = [song]
    return song
  }

  it('paints the colors of the cover playing', async () => {
    useThemeStore().setTheme('cover')
    sourceColorOf.mockResolvedValue(42)
    playing('http://test/cover.jpg')

    mount(useCoverTheme)
    await settle()

    expect(sourceColorOf).toHaveBeenCalledWith('http://test/cover.jpg')
    expect(paintRoot).toHaveBeenLastCalledWith({
      '--schemes-primary': `42-${useThemeStore().state.dark ? 'dark' : 'light'}`,
    })
  })

  it('falls back to the fixed colors when the cover cannot be read', async () => {
    useThemeStore().setTheme('cover')
    sourceColorOf.mockResolvedValue(null)
    playing('http://test/broken.jpg')

    mount(useCoverTheme)
    await settle()

    expect(paintRoot).toHaveBeenLastCalledWith(null)
  })

  it('leaves the colors alone with a fixed scheme chosen', async () => {
    useThemeStore().setTheme('pink')
    playing('http://test/cover.jpg')

    mount(useCoverTheme)
    await settle()

    expect(sourceColorOf).not.toHaveBeenCalled()
    expect(paintRoot).toHaveBeenLastCalledWith(null)
  })

  it('tints with a cover of its own', async () => {
    sourceColorOf.mockResolvedValue(7)
    let tint = ref<string | null>(null)

    mount(() => {
      tint = useCoverTint(ref('http://test/album.jpg'))
    })
    await settle()

    expect(tint.value).toBe(`7-${useThemeStore().state.dark ? 'dark' : 'light'}`)
  })
})
