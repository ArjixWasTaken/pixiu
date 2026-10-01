import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './AlbumTable.vue'

const virtualScrollerStub = {
  name: 'VirtualScrollerStub',
  props: ['items', 'itemHeight'],
  template: '<div><template v-for="item in items" :key="item.id"><slot :item="item" /></template></div>',
}

describe('albumTable.vue', () => {
  const h = createHarness()

  const renderWithAlbums = (count = 3) => {
    const albums = count === 0 ? [] : h.factory('album').make(count)
    return {
      albums,
      ...h.render(Component, {
        props: { albums, field: 'name' as const, order: 'asc' as const },
        global: { stubs: { VirtualScroller: virtualScrollerStub } },
      }),
    }
  }

  it('renders a sort-by-name header', () => {
    renderWithAlbums(3)

    screen.getByRole('button', { name: 'Name, sorted ascending' })
  })

  it('emits sort with toggled order when a header is clicked', async () => {
    const { emitted } = renderWithAlbums(0)

    await h.user.click(screen.getByRole('button', { name: 'Name, sorted ascending' }))

    expect(emitted('sort')?.[0]).toEqual(['name', 'desc'])
  })

  it('offers to choose the columns', () => {
    renderWithAlbums(0)

    screen.getByRole('button', { name: 'Columns' })
  })
})
