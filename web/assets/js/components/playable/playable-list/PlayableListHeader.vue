<template>
  <div v-if="showFilter || config.sortable" class="sort-bar" data-testid="list-toolbar">
    <ListFilter v-if="showFilter" />
    <span class="flex-1" />
    <M3MenuPopover v-if="config.sortable" v-model:open="open" menu-class="sort-menu">
      <template #anchor>
        <M3Chip
          :icon="sortOrder === 'asc' ? 'arrow_upward' : 'arrow_downward'"
          :title="`Sorted by ${currentLabel}, ${sortOrder === 'asc' ? 'ascending' : 'descending'}`"
          data-testid="sort-chip"
          @click="open = !open"
        >
          {{ currentLabel }}
        </M3Chip>
      </template>
      <M3MenuItem
        v-for="option in options"
        :key="option.label"
        :label="option.label"
        :selected="isCurrent(option.field)"
        data-testid="sort-menu-item"
        tag="div"
        @click="sort(option.field)"
      >
        <template #trailing>
          <M3Icon
            v-if="isCurrent(option.field)"
            :name="sortOrder === 'asc' ? 'arrow_upward' : 'arrow_downward'"
            :size="18"
          />
        </template>
      </M3MenuItem>
    </M3MenuPopover>
  </div>
</template>

<script setup lang="ts">
import type { Ref } from 'vue'
import { computed, ref } from 'vue'
import { arrayify, requireInjection } from '@/utils/helpers'
import {
  PlayableListConfigKey,
  PlayableListSortFieldKey,
  PlayableListSortOrderKey,
  PlayablesKey,
} from '@/config/symbols'

import M3Chip from '@/components/m3/M3Chip.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'
import M3MenuPopover from '@/components/m3/M3MenuPopover.vue'
import ListFilter from '@/components/ui/ListFilter.vue'

const emit = defineEmits<{
  (e: 'sort', field: MaybeArray<PlayableListSortField>, order: SortOrder): void
}>()

const [sortField, setSortField] =
  requireInjection<[Ref<MaybeArray<PlayableListSortField>>, Closure]>(PlayableListSortFieldKey)
const [sortOrder, setSortOrder] = requireInjection<[Ref<SortOrder>, Closure]>(PlayableListSortOrderKey)
const [config] = requireInjection<[Partial<PlayableListConfig>]>(PlayableListConfigKey, [{}])

const [allPlayables] = requireInjection<[Ref<Playable[]>]>(PlayablesKey, [ref([])])

const showFilter = computed(() => config.filterable && allPlayables.value.length > 0)

const open = ref(false)

const options: Array<{ label: string; field: MaybeArray<PlayableListSortField> }> = [
  { label: 'Title', field: 'title' },
  { label: 'Artist', field: 'artist_name' },
  { label: 'Album', field: 'album_name' },
  { label: 'Track', field: 'track' },
  { label: 'Year', field: 'year' },
  { label: 'Genre', field: 'genre' },
  { label: 'Duration', field: 'length' },
  { label: 'Date added', field: 'created_at' },
]

const isCurrent = (field: MaybeArray<PlayableListSortField>) =>
  arrayify(field).join() === arrayify(sortField.value).join()

// Unsorted, or by position: the list's own order (a playlist's, the queue's).
const currentLabel = computed(() => options.find(({ field }) => isCurrent(field))?.label ?? 'Default order')

const sort = (field: MaybeArray<PlayableListSortField>) => {
  setSortOrder(isCurrent(field) && sortOrder.value === 'asc' ? 'desc' : 'asc')
  setSortField(field)
  open.value = false

  emit('sort', field, sortOrder.value)
}
</script>

<style scoped>
/* Filter and sort, kept in reach while the list scrolls. */
.sort-bar {
  position: sticky;
  top: var(--sticky-top, 0);
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 24px;
  background: var(--schemes-surface);
}

:deep(.sort-menu) {
  min-width: 200px;
}
</style>
