<template>
  <div class="album-table-wrap relative flex flex-col flex-1 overflow-auto" data-testid="album-table">
    <div class="album-table-header list-table-row sticky top-0 z-2">
      <span class="name cover-aligned">
        <TableSortButton :active="field === 'name'" :order label="Name" @sort="onSort('name')" />
      </span>
      <span v-if="shouldShowColumn('artist')" class="artist">
        <TableSortButton :active="field === 'artist_name'" :order label="Artist" @sort="onSort('artist_name')" />
      </span>
      <span v-if="shouldShowColumn('time')" class="time">
        <TableSortButton :active="field === 'length'" :order label="Time" @sort="onSort('length')" />
      </span>
      <span v-if="shouldShowColumn('year')" class="year">
        <TableSortButton :active="field === 'year'" :order label="Year" @sort="onSort('year')" />
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
        <TableColumnsMenu :columns :config="albumTableColumnConfig" />
      </span>
    </div>

    <VirtualScroller :items="albums" :item-height="rowHeight" @scrolled-to-end="$emit('scrolled-to-end')">
      <template #default="{ item }: { item: Album }">
        <AlbumRow :album="item" @toggle-favorite="emit('toggle-favorite', $event)" />
      </template>
    </VirtualScroller>
  </div>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { useSizeVariable } from '@/composables/useSizeVariable'
import { albumTableColumnConfig } from '@/config/tables'

import VirtualScroller from '@/components/ui/VirtualScroller.vue'
import AlbumRow from '@/components/album/AlbumRow.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import TableColumnsMenu from '@/components/ui/TableColumnsMenu.vue'
import TableSortButton from '@/components/ui/TableSortButton.vue'

const props = defineProps<{
  albums: Album[]
  field: AlbumListSortField
  order: SortOrder
}>()

const emit = defineEmits<{
  (e: 'sort', field: AlbumListSortField, order: SortOrder): void
  (e: 'toggle-favorite', album: Album): void
  (e: 'scrolled-to-end'): void
}>()

const { shouldShowColumn } = useTableColumnVisibility(albumTableColumnConfig)
const rowHeight = useSizeVariable('--m3-row-height', 72)
const { field, order } = toRefs(props)

const columns: { name: AlbumTableColumnName; label: string }[] = [
  { name: 'name', label: 'Name' },
  { name: 'artist', label: 'Artist' },
  { name: 'time', label: 'Time' },
  { name: 'year', label: 'Year' },
  { name: 'rating', label: 'Rating' },
  { name: 'favorite', label: 'Favorite' },
]

const onSort = (clicked: AlbumListSortField) => {
  const nextOrder: SortOrder = field.value === clicked && order.value === 'asc' ? 'desc' : 'asc'
  emit('sort', clicked, nextOrder)
}
</script>
