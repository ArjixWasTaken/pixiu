import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ArtistTable.vue'

const virtualScrollerStub = {
  name: 'VirtualScrollerStub',
  props: ['items', 'itemHeight'],
  template: '<div><template v-for="item in items" :key="item.id"><slot :item="item" /></template></div>',
}

describe('artistTable.vue', () => {
  const h = createHarness()

  const renderWithArtists = (count = 3) => {
    const artists = count === 0 ? [] : h.factory('artist').make(count)
    return {
      artists,
      ...h.render(Component, {
        props: { artists, field: 'name' as const, order: 'asc' as const },
        global: { stubs: { VirtualScroller: virtualScrollerStub } },
      }),
    }
  }

  it('renders a sort-by-name header', () => {
    renderWithArtists(3)

    screen.getByRole('button', { name: 'Name, sorted ascending' })
  })

  it('emits sort with toggled order when a header is clicked', async () => {
    const { emitted } = renderWithArtists(0)

    await h.user.click(screen.getByRole('button', { name: 'Name, sorted ascending' }))

    expect(emitted('sort')?.[0]).toEqual(['name', 'desc'])
  })

  it('offers to choose the columns', () => {
    renderWithArtists(0)

    screen.getByRole('button', { name: 'Columns' })
  })
})
