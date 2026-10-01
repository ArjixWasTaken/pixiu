import { computed } from 'vue'
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core'
import { useUserStorage } from '@/composables/useUserStorage'

interface Options<T extends string> {
  storageKey: string
  validColumns: readonly T[]
  defaultColumns: readonly T[]
  alwaysVisible: readonly T[]
  /**
   * When true, the column-visibility preference only applies at md+ breakpoints.
   * Below md, all columns report visible (the caller is expected to hide them
   * via CSS instead). Defaults to false.
   */
  responsive?: boolean
}

export const useTableColumnVisibility = <T extends string>({
  storageKey,
  validColumns,
  defaultColumns,
  alwaysVisible,
  responsive = false,
}: Options<T>) => {
  // The same key in several places stays one value: VueUse keeps them in step.
  const stored = useUserStorage<T[]>(storageKey, [...defaultColumns])

  /** The stored columns that still exist, and the ones always shown. */
  const visibleColumns = computed(() =>
    Array.from(new Set([...stored.value.filter(column => validColumns.includes(column)), ...alwaysVisible])),
  )

  const isConfigurable = () => {
    if (!responsive) {
      return true
    }

    return useBreakpoints(breakpointsTailwind).isGreaterOrEqual('md')
  }

  const shouldShowColumn = (name: T) => {
    if (!isConfigurable()) {
      return true
    }

    return visibleColumns.value.includes(name)
  }

  const toggleColumn = (column: T) => {
    if (alwaysVisible.includes(column)) {
      return
    }

    let next = visibleColumns.value

    if (next.includes(column)) {
      next = next.filter(c => c !== column)
    } else {
      next = [...next, column]
    }

    stored.value = next
  }

  const isToggleable = (column: T) => !alwaysVisible.includes(column)

  return {
    shouldShowColumn,
    toggleColumn,
    isToggleable,
    isConfigurable,
  }
}
