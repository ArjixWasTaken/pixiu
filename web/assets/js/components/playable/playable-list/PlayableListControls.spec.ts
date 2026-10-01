import { screen } from '@testing-library/vue'
import { merge } from 'lodash-es'
import { ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { FilteredPlayablesKey, PlayablesKey, SelectedPlayablesKey } from '@/config/symbols'
import Component from './PlayableListControls.vue'

describe('playableListControls.vue', () => {
  const h = createHarness()

  const renderComponent = (selectedCount = 1, configOverrides: Partial<PlayableListControlsConfig> = {}) => {
    const songs = h.factory('song').make(5)
    const config: PlayableListControlsConfig = merge(
      {
        addTo: {
          queue: true,
          favorites: true,
        },
        clearQueue: true,
        deletePlaylist: true,
        refresh: true,
        filter: true,
      },
      configOverrides,
    )

    return h.render(Component, {
      global: {
        provide: {
          [<symbol>PlayablesKey]: [ref(songs)],
          [<symbol>FilteredPlayablesKey]: [ref(songs)],
          [<symbol>SelectedPlayablesKey]: [ref(songs.slice(0, selectedCount))],
        },
      },
      props: {
        config,
      },
    })
  }

  it.each([[0], [1]])('plays all in order if %s songs are selected', async (selectedCount: number) => {
    const { emitted } = renderComponent(selectedCount)

    await h.user.click(screen.getByRole('button', { name: 'Play' }))

    expect(emitted()['play-all'][0]).toEqual([false])
  })

  it.each([[0], [1]])('shuffles all if %s songs are selected', async (selectedCount: number) => {
    const { emitted } = renderComponent(selectedCount)

    await h.user.click(screen.getByRole('button', { name: 'Shuffle' }))

    expect(emitted()['play-all'][0]).toEqual([true])
  })

  it('plays the selection in order if more than one song is selected', async () => {
    const { emitted } = renderComponent(2)

    await h.user.click(screen.getByRole('button', { name: 'Play selected' }))

    expect(emitted()['play-selected'][0]).toEqual([false])
  })

  it('shuffles the selection if more than one song is selected', async () => {
    const { emitted } = renderComponent(2)

    await h.user.click(screen.getByRole('button', { name: 'Shuffle' }))

    expect(emitted()['play-selected'][0]).toEqual([true])
  })

  it('clears queue', async () => {
    const { emitted } = renderComponent(0)

    await h.user.click(screen.getByTitle('Clear current queue'))

    expect(emitted()['clear-queue']).toBeTruthy()
  })
})
