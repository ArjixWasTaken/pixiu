import { describe, expect, it } from 'vite-plus/test'
import { ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { CurrentStreamableKey } from '@/config/symbols'
import { cache } from '@/services/cache'
import Router from '@/router'
import { useNowPlaying } from '@/composables/useNowPlaying'
import Component from './FooterPlayableInfo.vue'

describe('footerPlayableInfo.vue', () => {
  const h = createHarness()

  it('renders with no current playable', () => expect(h.render(Component).html()).toMatchSnapshot())

  it('renders with current playable', () => {
    const song = h.factory('song').make({
      title: 'Fahrstuhl zum Mond',
      album_cover: 'https://via.placeholder.com/150',
      playback_state: 'Playing',
      artist_id: 'led-zeppelin',
      artist_name: 'Led Zeppelin',
    })

    expect(
      h
        .render(Component, {
          global: {
            provide: {
              [<symbol>CurrentStreamableKey]: ref(song),
            },
          },
        })
        .html(),
    ).toMatchSnapshot()
  })

  it('toggles the now playing panel on thumbnail click', async () => {
    const song = h.factory('song').make({ title: 'Test Song', playback_state: 'Playing' })
    const { open, close } = useNowPlaying()
    close()

    const { container } = h.render(Component, {
      global: {
        provide: {
          [<symbol>CurrentStreamableKey]: ref(song),
        },
      },
    })

    await h.user.click(container.querySelector('.album-thumb') as HTMLElement)

    expect(open.value).toBe(true)
    close()
  })

  it('does not navigate or set scroll intent when no playable', async () => {
    const goMock = h.mock(Router, 'go')
    const setMock = h.mock(cache, 'set')

    const { container } = h.render(Component)

    const thumb = container.querySelector('.album-thumb') as HTMLElement
    await h.user.click(thumb)

    expect(goMock).not.toHaveBeenCalled()
    expect(setMock).not.toHaveBeenCalled()
  })
})
