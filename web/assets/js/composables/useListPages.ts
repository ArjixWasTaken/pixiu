import type { InfiniteData } from '@tanstack/vue-query'
import { keepPreviousData, useInfiniteQuery } from '@tanstack/vue-query'
import type { MaybeRefOrGetter } from 'vue'
import { computed, toValue, watch } from 'vue'
import { queryClient } from '@/services/queryClient'
import { logger } from '@/utils/logger'
import { useErrorHandler } from '@/composables/useErrorHandler'

/** A page of a long list, and where the next one starts (`null`: there is none). */
export interface ListPage<T> {
  items: T[]
  nextCursor: string | null
}

/** Drops items from the pages kept under `key`, and the keys below it (each sort, each filter). */
export const dropFromListPages = (key: readonly unknown[], ids: Iterable<string>) => {
  const dropped = new Set(ids)

  queryClient.setQueriesData<InfiniteData<ListPage<{ id: string }>>>({ queryKey: key }, data =>
    data?.pages
      ? {
          ...data,
          pages: data.pages.map(page => ({ ...page, items: page.items.filter(({ id }) => !dropped.has(id)) })),
        }
      : data,
  )
}

/**
 * A long list the server sends a page at a time, kept by TanStack Query under
 * `key`: each sort and filter apart, fetched again once stale.
 */
export const useListPages = <T extends { id: string }>(
  key: MaybeRefOrGetter<readonly unknown[]>,
  fetchPage: (cursor: string) => Promise<ListPage<T>>,
  {
    enabled = true,
    keepPrevious = false,
  }: {
    enabled?: MaybeRefOrGetter<boolean>
    /**
     * Whether the list shows what it had while another key (a new sort) loads,
     * rather than nothing: for one list in another order, not for another list.
     */
    keepPrevious?: boolean
  } = {},
) => {
  const queryKey = computed(() => toValue(key))

  const query = useInfiniteQuery({
    queryKey,
    queryFn: ({ pageParam }) => fetchPage(pageParam),
    initialPageParam: '',
    getNextPageParam: page => page.nextCursor ?? undefined,
    enabled: computed(() => toValue(enabled)),
    placeholderData: keepPrevious ? keepPreviousData : undefined,
  })

  /**
   * What came so far, page after page. Set to fewer (some were deleted, say),
   * it keeps those only.
   */
  const items = computed<T[]>({
    get: () => query.data.value?.pages.flatMap(page => page.items) ?? [],
    set: kept => {
      const keptIds = new Set(kept.map(({ id }) => id))
      dropFromListPages(
        queryKey.value,
        items.value.filter(({ id }) => !keptIds.has(id)).map(({ id }) => id),
      )
    },
  })

  /** Whether the list couldn't load, with nothing to show: the screen says so, and offers to try again. */
  const loadFailed = computed(() => query.isError.value && !query.isFetching.value && items.value.length === 0)

  // Only a later page failing needs a toast; with nothing to show, the screen tells it (`loadFailed`).
  const { handleHttpError } = useErrorHandler()
  watch(query.error, error => error && (items.value.length ? handleHttpError(error) : logger.error(error)))

  /** The next page, unless it's on its way or there is none. */
  const fetchMore = async () => {
    if (query.hasNextPage.value && !query.isFetchingNextPage.value) {
      await query.fetchNextPage()
    }
  }

  return { ...query, items, loadFailed, fetchMore }
}
