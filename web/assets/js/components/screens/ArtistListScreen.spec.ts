import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { setViewport } from '@/composables/useViewport'
import { useArtistStore } from '@/stores/artistStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import Component from './ArtistListScreen.vue'

const artistGridStub = {
  template: '<div data-testid="artist-grid"><div v-for="(a, i) in artists" :key="i" data-testid="artist-card" /></div>',
  props: ['artists'],
  methods: { scrollToTop() {} },
}

const artistTableStub = {
  template: '<div data-testid="artist-table-stub" />',
  props: ['artists', 'field', 'order'],
}

describe('artistListScreen.vue', () => {
  const h = createHarness()

  beforeEach(() => setViewport({ mobile: false }))
  afterEach(() => setViewport({ mobile: true, wide: true }))

  const renderComponent = async () => {
    const artists = h.factory('artist').make(9)
    const paginateMock = h
      .mock(useArtistStore(), 'paginate')
      .mockResolvedValue({ items: useArtistStore().syncWithVault(artists), nextCursor: 'next-cursor-token' })

    const rendered = h.render(Component, {
      global: {
        stubs: {
          ArtistGrid: artistGridStub,
          ArtistTable: artistTableStub,
        },
      },
    })

    // An empty library has nothing to fetch.
    useCommonStore().state.song_length && (await waitFor(() => expect(paginateMock).toHaveBeenCalled()))
    await h.tick(2)

    return {
      ...rendered,
      paginateMock,
    }
  }

  it('renders', async () => {
    await renderComponent()
    expect(screen.getAllByTestId('artist-card')).toHaveLength(9)
  })

  it('shows a message when the library is empty', async () => {
    useCommonStore().state.song_length = 0

    await renderComponent()

    await waitFor(() => screen.getByTestId('screen-empty-state'))
  })

  it('renders the table when the view mode is table', async () => {
    usePreferenceStore().artists_view_mode = 'table'
    await renderComponent()

    expect(screen.queryByTestId('artist-grid')).toBeNull()
    screen.getByTestId('artist-table-stub')
  })

  it('switches between grid and table via the view mode toggle', async () => {
    usePreferenceStore().artists_view_mode = 'grid'
    await renderComponent()

    screen.getByTestId('artist-grid')
    expect(screen.queryByTestId('artist-table-stub')).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Table' }))
    await waitFor(() => {
      screen.getByTestId('artist-table-stub')
      expect(screen.queryByTestId('artist-grid')).toBeNull()
    })

    await h.user.click(screen.getByRole('button', { name: 'Grid' }))
    await waitFor(() => {
      screen.getByTestId('artist-grid')
      expect(screen.queryByTestId('artist-table-stub')).toBeNull()
    })
  })

  it('shows all or only favorites upon toggling the button', async () => {
    const { paginateMock } = await renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Favorites only' }))

    await waitFor(() =>
      expect(paginateMock).toHaveBeenNthCalledWith(2, {
        favorites_only: true,
        cursor: '',
        order: 'asc',
        sort: 'name',
      }),
    )

    // Back to all of them: the list kept from before, not fetched again.
    await h.user.click(screen.getByRole('button', { name: 'Favorites only' }))
    await h.tick(2)

    expect(paginateMock).toHaveBeenCalledTimes(2)
  })

  it('filters out unfavorited artists in favorites mode', async () => {
    const artists = useArtistStore().syncWithVault(h.factory('artist').make({ favorite: true }, 5))
    h.mock(useArtistStore(), 'paginate').mockResolvedValue({ items: artists, nextCursor: null })

    h.render(Component, {
      global: {
        stubs: {
          ArtistGrid: artistGridStub,
          ArtistTable: artistTableStub,
        },
      },
    })

    await h.tick(2)

    usePreferenceStore().artists_favorites_only = true

    await waitFor(() => expect(screen.getAllByTestId('artist-card')).toHaveLength(5))

    artists[0].favorite = false
    await h.tick()

    expect(screen.getAllByTestId('artist-card')).toHaveLength(4)
  })

  it('shows empty state when no favorite artists', async () => {
    h.mock(useArtistStore(), 'paginate').mockResolvedValue({ items: [], nextCursor: null })
    usePreferenceStore().artists_favorites_only = true

    h.render(Component, {
      global: {
        stubs: {
          ArtistGrid: artistGridStub,
          ArtistTable: artistTableStub,
        },
      },
    })

    await h.tick(2)

    await waitFor(() => {
      const emptyState = screen.getByTestId('screen-empty-state')
      expect(emptyState.textContent).toContain('No favorite artists')
    })
  })
})
