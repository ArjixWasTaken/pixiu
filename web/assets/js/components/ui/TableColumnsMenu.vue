<template>
  <M3MenuPopover :min-width="200">
    <template #anchor>
      <M3IconButton :icon-size="20" icon="view_column" label="Columns" title="Columns" />
    </template>
    <M3MenuCheckboxItem
      v-for="column in columns"
      :key="column.name"
      :disabled="!isToggleable(column.name)"
      :label="column.label"
      :model-value="shouldShowColumn(column.name)"
      @update:model-value="toggleColumn(column.name)"
    />
  </M3MenuPopover>
</template>

<script generic="T extends string" lang="ts" setup>
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3MenuCheckboxItem from '@/components/m3/M3MenuCheckboxItem.vue'
import M3MenuPopover from '@/components/m3/M3MenuPopover.vue'

/** Which of a table's columns show; the ones always shown can't be turned off. */
const props = defineProps<{
  config: Parameters<typeof useTableColumnVisibility<T>>[0]
  columns: { name: T; label: string }[]
}>()

const { shouldShowColumn, toggleColumn, isToggleable } = useTableColumnVisibility<T>(props.config)
</script>
