<template>
  <div class="artist-table-wrap relative flex flex-col flex-1 overflow-auto" data-testid="artist-table">
    <div class="artist-table-header list-table-row sticky top-0 z-2">
      <span class="name cover-aligned">
        <TableSortButton :active="field === 'name'" :order label="Name" @sort="onSort('name')" />
      </span>
      <span v-if="shouldShowColumn('rating')" class="rating">
        <TableSortButton :active="field === 'rating'" :order label="Rating" @sort="onSort('rating')" />
      </span>
      <span v-if="shouldShowColumn('favorite')" class="favorite">
        <TableSortButton :active="field === 'favorite'" :order label="Favorite" @sort="onSort('favorite')">
          <M3Icon :size="18" fill name="favorite" />
        </TableSortButton>
      </span>
      <span class="extra">
        <TableColumnsMenu :columns :config="artistTableColumnConfig" />
      </span>
    </div>

    <VirtualScroller :items="artists" :item-height="rowHeight" @scrolled-to-end="$emit('scrolled-to-end')">
      <template #default="{ item }: { item: Artist }">
        <ArtistRow :artist="item" @toggle-favorite="emit('toggle-favorite', $event)" />
      </template>
    </VirtualScroller>
  </div>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { useSizeVariable } from '@/composables/useSizeVariable'
import { artistTableColumnConfig } from '@/config/tables'

import VirtualScroller from '@/components/ui/VirtualScroller.vue'
import ArtistRow from '@/components/artist/ArtistRow.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import TableColumnsMenu from '@/components/ui/TableColumnsMenu.vue'
import TableSortButton from '@/components/ui/TableSortButton.vue'

const props = defineProps<{
  artists: Artist[]
  field: ArtistListSortField
  order: SortOrder
}>()

const emit = defineEmits<{
  (e: 'sort', field: ArtistListSortField, order: SortOrder): void
  (e: 'toggle-favorite', artist: Artist): void
  (e: 'scrolled-to-end'): void
}>()

const { shouldShowColumn } = useTableColumnVisibility(artistTableColumnConfig)
const rowHeight = useSizeVariable('--m3-row-height', 72)
const { field, order } = toRefs(props)

const columns: { name: ArtistTableColumnName; label: string }[] = [
  { name: 'name', label: 'Name' },
  { name: 'rating', label: 'Rating' },
  { name: 'favorite', label: 'Favorite' },
]

const onSort = (clicked: ArtistListSortField) => {
  const nextOrder: SortOrder = field.value === clicked && order.value === 'asc' ? 'desc' : 'asc'
  emit('sort', clicked, nextOrder)
}
</script>
