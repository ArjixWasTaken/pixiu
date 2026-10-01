import { screen } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { assertOpenModal } from '@/__tests__/assertions'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useQueueStore } from '@/stores/queueStore'
import { arrayify } from '@/utils/helpers'
import { usePlayableStore } from '@/stores/playableStore'
import CreatePlaylistForm from '@/components/playlist/CreatePlaylistForm.vue'

const openModalMock = vi.fn()

vi.mock('@/composables/useModal', () => ({
  useModal: () => ({
    openModal: openModalMock,
  }),
}))

import Component from './AddToMenu.vue'

describe('addToMenu.vue', () => {
  const h = createHarness({
    beforeEach: () => openModalMock.mockClear(),
  })

  const renderComponent = (customConfig: Partial<AddToMenuConfig> = {}) => {
    const playables = h.factory('song').make(5)

    const config: AddToMenuConfig = {
      queue: true,
      favorites: true,
    }

    const rendered = h.render(Component, {
      props: {
        playables,
        config: { ...config, ...customConfig },
        showing: true,
      },
    })

    return {
      ...rendered,
      playables,
    }
  }

  it('renders', () => {
    usePlaylistStore().state.playlists = [
      h.factory('playlist').make({ name: 'Foo' }),
      h.factory('playlist').make({ name: 'Bar' }),
      h.factory('playlist').make({ name: 'Baz' }),
    ]

    expect(renderComponent().html()).toMatchSnapshot()
  })

  it.each<[keyof AddToMenuConfig, string | string[]]>([
    ['queue', ['queue-after-current', 'queue-bottom', 'queue-top', 'queue']],
    ['favorites', 'add-to-favorites'],
  ])('renders disabling %s config', (configKey: keyof AddToMenuConfig, testIds: string | string[]) => {
    renderComponent({ [configKey]: false })
    arrayify(testIds).forEach(id => expect(screen.queryByTestId(id)).toBeNull())
  })

  it.each<[string, string, MethodOf<Required<ReturnType<typeof useQueueStore>>>]>([
    ['after current', 'queue-after-current', 'queueAfterCurrent'],
    ['to top', 'queue-top', 'queueToTop'],
    ['to bottom', 'queue-bottom', 'queue'],
  ])(
    'queues songs %s',
    async (_: string, testId: string, queueMethod: MethodOf<Required<ReturnType<typeof useQueueStore>>>) => {
      useQueueStore().state.playables = h.factory('song').make(5)
      usePlayableStore().syncWithVault(useQueueStore().state.playables)
      useQueueStore().state.playables[2].playback_state = 'Playing'

      const mock = h.mock(useQueueStore(), queueMethod)
      const { playables } = renderComponent()

      await h.user.click(screen.getByTestId(testId))

      expect(mock).toHaveBeenCalledWith(playables)
    },
  )

  it('adds songs to Favorites', async () => {
    const mock = h.mock(usePlayableStore(), 'favorite')
    const { playables } = renderComponent()

    await h.user.click(screen.getByTestId('add-to-favorites'))

    expect(mock).toHaveBeenCalledWith(playables)
  })

  it('chooses with Enter, as with a click', async () => {
    const mock = h.mock(usePlayableStore(), 'favorite')
    const { playables } = renderComponent()

    screen.getByTestId('add-to-favorites').focus()
    await h.user.keyboard('{Enter}')

    expect(mock).toHaveBeenCalledWith(playables)
  })

  it('adds songs to existing playlist', async () => {
    const mock = h.mock(usePlaylistStore(), 'addContent')
    usePlaylistStore().state.playlists = h.factory('playlist').make(3)
    const { playables } = renderComponent()

    await h.user.click(screen.getAllByTestId('add-to-playlist')[1])

    expect(mock).toHaveBeenCalledWith(usePlaylistStore().state.playlists[1], playables)
  })

  it('creates playlist from selected songs', async () => {
    const { playables } = renderComponent()

    await h.user.click(screen.getByText('New playlist…'))

    await assertOpenModal(openModalMock, CreatePlaylistForm, { folder: null, playables })
  })
})
