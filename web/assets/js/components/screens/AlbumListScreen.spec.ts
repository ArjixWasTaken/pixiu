import { screen, waitFor } from '@testing-library/vue'
import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { setViewport } from '@/composables/useViewport'
import { useAlbumStore } from '@/stores/albumStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { logger } from '@/utils/logger'
import Component from './AlbumListScreen.vue'

const albumGridStub = {
  template: '<div data-testid="album-grid"><div v-for="(a, i) in albums" :key="i" data-testid="album-card" /></div>',
  props: ['albums', 'showReleaseYear'],
  methods: { scrollToTop() {} },
}

const albumTableStub = {
  template: '<div data-testid="album-table" />',
  props: ['albums', 'field', 'order'],
}

describe('albumListScreen.vue', () => {
  const h = createHarness()

  beforeEach(() => setViewport({ mobile: false }))
  afterEach(() => setViewport({ mobile: true, wide: true }))

  const renderComponent = async () => {
    const albums = h.factory('album').make(9)
    const paginateMock = h
      .mock(useAlbumStore(), 'paginate')
      .mockResolvedValue({ items: useAlbumStore().syncWithVault(albums), nextCursor: 'next-cursor-token' })

    const rendered = h.render(Component, {
      global: {
        stubs: {
          AlbumGrid: albumGridStub,
          AlbumTable: albumTableStub,
        },
      },
    })

    // An empty library has nothing to fetch.
    useCommonStore().state.song_length && (await waitFor(() => expect(paginateMock).toHaveBeenCalled()))
    await h.tick(2)

    return {
      rendered,
      paginateMock,
    }
  }

  it('renders', async () => {
    await renderComponent()
    expect(screen.getAllByTestId('album-card')).toHaveLength(9)
  })

  it('shows a message when the library is empty', async () => {
    useCommonStore().state.song_length = 0
    await renderComponent()

    await waitFor(() => screen.getByTestId('screen-empty-state'))
  })

  it('says when the albums couldn’t load, and tries again', async () => {
    h.mock(logger, 'error')
    const paginateMock = h.mock(useAlbumStore(), 'paginate').mockRejectedValueOnce(new Error('offline'))
    paginateMock.mockResolvedValueOnce({
      items: useAlbumStore().syncWithVault(h.factory('album').make(3)),
      nextCursor: null,
    })
    h.render(Component, { global: { stubs: { AlbumGrid: albumGridStub, AlbumTable: albumTableStub } } })

    await screen.findByText(/Couldn’t load the albums\./)

    await h.user.click(screen.getByRole('button', { name: 'Try again' }))

    await waitFor(() => expect(screen.getAllByTestId('album-card')).toHaveLength(3))
  })

  it('renders the table when the view mode is table', async () => {
    usePreferenceStore().albums_view_mode = 'table'
    await renderComponent()

    expect(screen.queryByTestId('album-grid')).toBeNull()
    screen.getByTestId('album-table')
  })

  it('switches between grid and table via the view mode toggle', async () => {
    usePreferenceStore().albums_view_mode = 'grid'
    await renderComponent()

    screen.getByTestId('album-grid')
    expect(screen.queryByTestId('album-table')).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Table' }))
    await waitFor(() => {
      screen.getByTestId('album-table')
      expect(screen.queryByTestId('album-grid')).toBeNull()
    })

    await h.user.click(screen.getByRole('button', { name: 'Grid' }))
    await waitFor(() => {
      screen.getByTestId('album-grid')
      expect(screen.queryByTestId('album-table')).toBeNull()
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

  it('filters out unfavorited albums in favorites mode', async () => {
    const albums = useAlbumStore().syncWithVault(h.factory('album').make({ favorite: true }, 5))
    h.mock(useAlbumStore(), 'paginate').mockResolvedValue({ items: albums, nextCursor: null })

    h.render(Component, {
      global: {
        stubs: {
          AlbumGrid: albumGridStub,
          AlbumTable: albumTableStub,
        },
      },
    })

    await h.tick(2)

    usePreferenceStore().albums_favorites_only = true

    await waitFor(() => expect(screen.getAllByTestId('album-card')).toHaveLength(5))

    albums[0].favorite = false
    await h.tick()

    expect(screen.getAllByTestId('album-card')).toHaveLength(4)
  })

  it('shows empty state when no favorite albums', async () => {
    h.mock(useAlbumStore(), 'paginate').mockResolvedValue({ items: [], nextCursor: null })
    usePreferenceStore().albums_favorites_only = true

    h.render(Component, {
      global: {
        stubs: {
          AlbumGrid: albumGridStub,
          AlbumTable: albumTableStub,
        },
      },
    })

    await h.tick(2)

    await waitFor(() => {
      const emptyState = screen.getByTestId('screen-empty-state')
      expect(emptyState.textContent).toContain('No favorite albums')
    })
  })
})
