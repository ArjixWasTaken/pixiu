import { screen } from '@testing-library/vue'
import { ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { PlayableListConfigKey, PlayableListSortFieldKey, PlayableListSortOrderKey } from '@/config/symbols'
import PlayableListHeader from './PlayableListHeader.vue'

describe('playableListHeader.vue', () => {
  const h = createHarness()

  const renderComponent = (
    config: Partial<PlayableListConfig> = { sortable: true },
    sortField: PlayableListSortField = 'title',
    sortOrder: SortOrder = 'asc',
  ) => {
    const sortFieldRef = ref<MaybeArray<PlayableListSortField>>(sortField)
    const sortOrderRef = ref(sortOrder)

    const rendered = h.render(PlayableListHeader, {
      props: { contentType: 'songs' },
      global: {
        provide: {
          [<symbol>PlayableListConfigKey]: [config],
          [<symbol>PlayableListSortFieldKey]: [
            sortFieldRef,
            (value: PlayableListSortField) => (sortFieldRef.value = value),
          ],
          [<symbol>PlayableListSortOrderKey]: [sortOrderRef, (value: SortOrder) => (sortOrderRef.value = value)],
        },
      },
    })

    return { ...rendered, sortFieldRef, sortOrderRef }
  }

  it('shows the current sort field', () => {
    renderComponent({ sortable: true }, 'album_name')
    expect(screen.getByTestId('sort-chip').textContent).toContain('Album')
  })

  it('sorts by another field, ascending', async () => {
    const { emitted, sortFieldRef } = renderComponent()

    await h.user.click(screen.getByTestId('sort-chip'))
    await h.user.click(screen.getByText('Artist'))

    expect(sortFieldRef.value).toBe('artist_name')
    expect(emitted().sort[0]).toEqual(['artist_name', 'asc'])
  })

  it('flips the order when the current field is picked again', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByTestId('sort-chip'))
    await h.user.click(screen.getByRole('menuitem', { name: /Title/ }))

    expect(emitted().sort[0]).toEqual(['title', 'desc'])
  })

  it('is hidden for lists that cannot be sorted', () => {
    renderComponent({ sortable: false })
    expect(screen.queryByTestId('sort-chip')).toBeNull()
  })
})
