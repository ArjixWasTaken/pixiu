import { describe, expect, it, vi } from 'vite-plus/test'
import { waitFor } from '@testing-library/vue'
import { defineComponent } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { MessageToasterStub } from '@/__tests__/stubs'
import { queryClient } from '@/services/queryClient'
import { logger } from '@/utils/logger'
import type { ListPage } from './useListPages'
import { dropFromListPages, useListPages } from './useListPages'

describe('useListPages', () => {
  const h = createHarness()

  const item = (id: string) => ({ id })

  /** Two pages: a, b; then c, the last. */
  const fetchPage = vi.fn(
    async (cursor: string): Promise<ListPage<{ id: string }>> =>
      cursor === '' ? { items: [item('a'), item('b')], nextCursor: 'two' } : { items: [item('c')], nextCursor: null },
  )

  const mount = () => {
    let list!: ReturnType<typeof useListPages<{ id: string }>>
    h.render(defineComponent({ setup: () => void (list = useListPages(['things'], fetchPage)), render: () => null }))
    return list
  }

  it('fetches the first page, then each next one asked for, until the last', async () => {
    fetchPage.mockClear()
    const list = mount()

    await waitFor(() => expect(list.items.value.map(({ id }) => id)).toEqual(['a', 'b']))

    await list.fetchMore()
    expect(list.items.value.map(({ id }) => id)).toEqual(['a', 'b', 'c'])

    await list.fetchMore()
    expect(fetchPage.mock.calls).toEqual([[''], ['two']])
  })

  it('keeps only what is left when set to fewer (some deleted)', async () => {
    const list = mount()
    await waitFor(() => expect(list.items.value).toHaveLength(2))

    list.items.value = [item('b')]

    expect(list.items.value.map(({ id }) => id)).toEqual(['b'])
  })

  it('says it couldn’t load when nothing came, and loads on a retry', async () => {
    const toastError = h.mock(MessageToasterStub.value, 'error')
    h.mock(logger, 'error')
    fetchPage.mockRejectedValueOnce(new Error('offline'))
    const list = mount()

    await waitFor(() => expect(list.loadFailed.value).toBe(true))
    // The screen says it; a toast would say it twice.
    expect(toastError).not.toHaveBeenCalled()

    await list.refetch()

    expect(list.loadFailed.value).toBe(false)
    expect(list.items.value.map(({ id }) => id)).toEqual(['a', 'b'])
  })

  it('drops items from every list kept under a key', () => {
    const pages = (...ids: string[]) => ({ pages: [{ items: ids.map(item), nextCursor: null }], pageParams: [''] })
    queryClient.setQueryData(['things', { sort: 'name' }], pages('a', 'b'))
    queryClient.setQueryData(['things', { sort: 'date' }], pages('b', 'a'))

    dropFromListPages(['things'], ['a'])

    expect(queryClient.getQueryData(['things', { sort: 'name' }])).toEqual(pages('b'))
    expect(queryClient.getQueryData(['things', { sort: 'date' }])).toEqual(pages('b'))
  })
})
