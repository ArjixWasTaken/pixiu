<template>
  <M3MenuPopover v-model:open="open" menu-class="sort-menu">
    <template #anchor>
      <M3Chip :icon="order === 'asc' ? 'arrow_upward' : 'arrow_downward'" :title @click="open = !open">
        {{ currentLabel }}
      </M3Chip>
    </template>
    <M3MenuItem
      v-for="item in items"
      :key="item.label"
      :class="{ active: isCurrentField(item.field) }"
      :label="item.label"
      :selected="isCurrentField(item.field)"
      :title="`Sort by ${item.label}`"
      tag="div"
      @click="sort(item.field)"
    >
      <template #trailing>
        <M3Icon
          v-if="isCurrentField(item.field)"
          :name="order === 'asc' ? 'arrow_upward' : 'arrow_downward'"
          :size="18"
        />
      </template>
    </M3MenuItem>
  </M3MenuPopover>
</template>

<script generic="T extends SortField" lang="ts" setup>
import { computed, ref, toRefs } from 'vue'

import M3Chip from '@/components/m3/M3Chip.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'
import M3MenuPopover from '@/components/m3/M3MenuPopover.vue'

const props = defineProps<{
  items: BasicListSorterDropDownItem<T>[]
  field?: T
  order?: SortOrder
}>()

const emit = defineEmits<{ (e: 'sort', field: T, order: SortOrder): void }>()

const { field: currentField, order: currentOrder, items } = toRefs(props)

const open = ref(false)

const currentLabel = computed(() => {
  return items.value.find((item: BasicListSorterDropDownItem<T>) => item.field === currentField.value)?.label
})

const sort = (field: T) => {
  if (field === currentField.value) {
    // if clicking the same field, toggle the order
    emit('sort', field, currentOrder.value === 'asc' ? 'desc' : 'asc')
  } else {
    // otherwise, we do ascending order by default
    emit('sort', field, 'asc')
  }

  open.value = false
}

const isCurrentField = (field: T) => field === currentField.value

const title = computed(
  () => `Sorting by ${currentLabel.value}, ${currentOrder.value === 'asc' ? 'ascending' : 'descending'}`,
)
</script>

<style scoped>
:deep(.sort-menu) {
  min-width: 200px;
}
</style>
