<template>
  <article ref="container" class="relative">
    <M3Chip :icon="order === 'asc' ? 'arrow_upward' : 'arrow_downward'" :title @click="open = !open">
      {{ currentLabel }}
    </M3Chip>
    <M3Menu v-show="open" class="menu">
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
    </M3Menu>
  </article>
</template>

<script generic="T extends SortField" lang="ts" setup>
import { computed, ref, toRefs, useTemplateRef } from 'vue'
import { onClickOutside } from '@vueuse/core'

import M3Chip from '@/components/m3/M3Chip.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3Menu from '@/components/m3/M3Menu.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'

const props = defineProps<{
  items: BasicListSorterDropDownItem<T>[]
  field?: T
  order?: SortOrder
}>()

const emit = defineEmits<{ (e: 'sort', field: T, order: SortOrder): void }>()

const { field: currentField, order: currentOrder, items } = toRefs(props)

const open = ref(false)
const container = useTemplateRef('container')
onClickOutside(container, () => (open.value = false))

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
.menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  z-index: 30;
  min-width: 200px;
}
</style>
