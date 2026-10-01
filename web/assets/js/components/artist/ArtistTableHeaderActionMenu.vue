<template>
  <article>
    <Popover v-model:open="open" class="context-menu normal-case tracking-normal" placement="bottom-end">
      <template #anchor>
        <button class="w-full focus:text-(--schemes-primary)" title="Sort" type="button">
          <M3Icon name="swap_vert" />
        </button>
      </template>
      <menu>
        <li
          v-for="item in menuItems"
          :key="item.label"
          :class="field === item.field && 'active'"
          class="cursor-pointer group flex justify-between pl-3! hover:bg-(--schemes-primary)! hover:text-(--schemes-on-primary)!"
          @click="sort(item.field)"
        >
          <label class="w-4 mr-2.5 flex items-center" @click.stop="toggle(item.column)">
            <input
              :checked="shouldShowColumn(item.column)"
              :disabled="!isToggleable(item.column)"
              :title="isToggleable(item.column) ? `Click to toggle the ${item.label} column` : ''"
              class="disabled:opacity-20 disabled:cursor-not-allowed bg-(--schemes-on-surface) group-hover:border-(--schemes-on-primary) h-4 aspect-square rounded-sm checked:border-(--schemes-outline) checked:border-2 checked:bg-(--schemes-primary)"
              type="checkbox"
            />
          </label>
          <span class="flex-1 text-left">{{ item.label }}</span>
          <span class="icon hidden ml-3">
            <M3Icon v-if="order === 'asc'" name="arrow_upward" />
            <M3Icon v-else name="arrow_downward" />
          </span>
        </li>
      </menu>
    </Popover>
  </article>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { artistTableColumnConfig } from '@/config/tables'

import Popover from '@/components/ui/Popover.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

defineProps<{
  field: ArtistListSortField
  order: SortOrder
}>()

const emit = defineEmits<{ (e: 'sort', field: ArtistListSortField): void }>()

interface MenuItem {
  column: ArtistTableColumnName
  label: string
  field: ArtistListSortField
}

const { shouldShowColumn, toggleColumn, isToggleable } = useTableColumnVisibility(artistTableColumnConfig)

const open = ref(false)

const menuItems = computed<MenuItem[]>(() => [
  { column: 'name', label: 'Name', field: 'name' },
  { column: 'rating', label: 'Rating', field: 'rating' },
  { column: 'favorite', label: 'Favorite', field: 'favorite' },
])

const sort = (field: ArtistListSortField) => {
  emit('sort', field)
  open.value = false
}

const toggle = (column: ArtistTableColumnName) => {
  if (!isToggleable(column)) {
    return
  }

  toggleColumn(column)
  open.value = false
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.active {
  @apply bg-(--schemes-primary) text-(--schemes-on-primary);

  .icon {
    @apply block;
  }

  input {
    @apply border-(--schemes-on-primary)!;
  }
}
</style>
