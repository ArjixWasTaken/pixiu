import { fireEvent, screen } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useCommonStore } from '@/stores/commonStore'
import { useOverviewStore } from '@/stores/overviewStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import type { Events } from '@/config/events'
import { eventBus } from '@/utils/eventBus'
import Component from './HomeScreen.vue'

const openModalSpy = vi.fn()
vi.mock('@/composables/useModal', () => ({
  useModal: () => ({ openModal: openModalSpy, closeModal: vi.fn() }),
}))

const blockIdsInDom = (container: Element) =>
  Array.from(container.querySelectorAll<HTMLElement>('.home-sections > [data-testid]')).map(el => el.dataset.testid!)

describe('homeScreen.vue', () => {
  const h = createHarness()

  const renderComponent = async () => {
    await h.visit('/home')
    h.render(Component)
  }

  it('renders an empty state if no songs found', async () => {
    useCommonStore().state.song_length = 0
    h.mock(useOverviewStore(), 'fetch')

    h.render(Component)

    screen.getByTestId('screen-empty-state')
  })

  it('renders overview components if applicable', async () => {
    useCommonStore().state.song_length = 100
    // Still loading: every section shows, its skeleton in place (an empty one hides once loaded).
    const fetchOverviewMock = h.mock(useOverviewStore(), 'fetch').mockReturnValue(new Promise(() => {}))

    await renderComponent()

    expect(fetchOverviewMock).toHaveBeenCalled()

    ;[
      'most-played-songs',
      'recently-played-songs',
      'recently-added-albums',
      'recently-added-songs',
      'most-played-artists',
      'most-played-albums',
    ].forEach(id => screen.getByTestId(id))

    expect(screen.queryByTestId('screen-empty-state')).toBeNull()
  })

  it.each<[keyof Events]>([['SONGS_UPDATED'], ['SONGS_DELETED'], ['SONG_UPLOADED']])(
    'refreshes the overviews on %s event',
    async eventName => {
      const fetchOverviewMock = h.mock(useOverviewStore(), 'fetch')
      await renderComponent()

      eventBus.emit(eventName)

      expect(fetchOverviewMock).toHaveBeenCalled()
    },
  )

  it('renders the reorder trigger button when the library is not empty', () => {
    useCommonStore().state.song_length = 100
    h.mock(useOverviewStore(), 'fetch')

    h.render(Component)

    screen.getByTestId('reorder-home-blocks-btn')
  })

  it('hides the reorder trigger button on the empty state', () => {
    useCommonStore().state.song_length = 0
    h.mock(useOverviewStore(), 'fetch')

    h.render(Component)

    expect(screen.queryByTestId('reorder-home-blocks-btn')).toBeNull()
  })

  it('opens the ReorderBlocksModal with the canonical block summaries when the trigger is clicked', async () => {
    useCommonStore().state.song_length = 100
    h.mock(useOverviewStore(), 'fetch')
    openModalSpy.mockClear()

    h.render(Component)

    await fireEvent.click(screen.getByTestId('reorder-home-blocks-btn'))

    expect(openModalSpy).toHaveBeenCalledOnce()
    const [, props] = openModalSpy.mock.calls[0] as [unknown, { blocks: { id: string; label: string }[] }]
    expect(props.blocks).toEqual(
      expect.arrayContaining([
        { id: 'recently-played-songs', label: 'Recently played' },
        { id: 'random-artists', label: 'Random artists' },
      ]),
    )
  })

  it('honors usePreferenceStore().home_blocks_order when rendering blocks', () => {
    useCommonStore().state.song_length = 100
    h.mock(useOverviewStore(), 'fetch')
    usePreferenceStore().home_blocks_order = ['random-songs', 'recently-added-albums']

    const { container } = h.render(Component)
    const ids = blockIdsInDom(container)

    expect(ids[0]).toBe('random-songs')
    expect(ids[1]).toBe('recently-added-albums')
    expect(ids).toContain('most-played-songs')
  })

  it('leaves out the blocks hidden in usePreferenceStore().home_blocks_hidden', () => {
    useCommonStore().state.song_length = 100
    h.mock(useOverviewStore(), 'fetch')
    usePreferenceStore().home_blocks_order = []
    usePreferenceStore().home_blocks_hidden = ['random-songs']

    const { container } = h.render(Component)
    const ids = blockIdsInDom(container)

    expect(ids).not.toContain('random-songs')
    expect(ids).toContain('recently-added-albums')
    usePreferenceStore().home_blocks_hidden = []
  })
})
