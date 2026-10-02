import { ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { arrayify } from '@/utils/helpers'
import {
  FilteredPlayablesKey,
  PlayableListConfigKey,
  PlayableListContextKey,
  PlayableListSortFieldKey,
  PlayableListSortOrderKey,
  SelectedPlayablesKey,
} from '@/config/symbols'
import Component from './PlayableList.vue'

describe('playableList.vue', () => {
  const h = createHarness()

  const renderComponent = async (
    songs: MaybeArray<Playable>,
    config: Partial<PlayableListConfig> = {
      sortable: true,
      reorderable: true,
    },
    context: PlayableListContext = {
      type: 'Album',
    },
    selectedPlayables: Playable[] = [],
    sortField: PlayableListSortField = 'title',
    sortOrder: SortOrder = 'asc',
  ) => {
    songs = arrayify(songs)

    const sortFieldRef = ref(sortField)
    const sortOrderRef = ref(sortOrder)

    const rendered = (await h.visit('/songs')).render(Component, {
      global: {
        stubs: {
          VirtualScroller: h.stub(),
          PlayableListSorter: h.stub(),
          PlayableListHeader: h.stub(),
        },
        provide: {
          [<symbol>FilteredPlayablesKey]: [ref(songs)],
          [<symbol>SelectedPlayablesKey]: [ref(selectedPlayables), (value: Playable[]) => (selectedPlayables = value)],
          [<symbol>PlayableListConfigKey]: [config],
          [<symbol>PlayableListContextKey]: [context],
          [<symbol>PlayableListSortFieldKey]: [
            sortFieldRef,
            (value: PlayableListSortField) => (sortFieldRef.value = value),
          ],
          [<symbol>PlayableListSortOrderKey]: [sortOrderRef, (value: SortOrder) => (sortOrderRef.value = value)],
        },
      },
    })

    return {
      ...rendered,
      songs,
    }
  }

  it('renders', async () => {
    const { html } = await renderComponent(h.factory('song').make(5))
    expect(html()).toMatchSnapshot()
  })

  it('acts on Enter and Ctrl+A only when a song has focus', async () => {
    const { container, emitted } = await renderComponent(h.factory('song').make(3))
    const list = container.querySelector<HTMLElement>('[data-testid="song-list"]')!

    // A field in the list keeps its keys: an "a" is typed, Enter is its own.
    const field = document.createElement('input')
    list.prepend(field)
    const typed = new KeyboardEvent('keydown', { key: 'a', bubbles: true, cancelable: true })
    field.dispatchEvent(typed)
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))

    expect(typed.defaultPrevented).toBe(false)
    expect(emitted()['press:enter']).toBeUndefined()

    // A song row's Enter is the list's.
    const row = document.createElement('article')
    row.className = 'song-item'
    list.append(row)
    row.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))

    expect(emitted()['press:enter']).toHaveLength(1)
  })
})
