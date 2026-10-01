<template>
  <button
    :aria-label="
      active ? `${label}, sorted ${order === 'asc' ? 'ascending' : 'descending'}` : `Sort by ${label.toLowerCase()}`
    "
    :class="{ active }"
    class="table-sort-button m3-state m3-label-medium"
    type="button"
    @click="emit('sort')"
  >
    <slot>{{ label }}</slot>
    <M3Icon v-if="active" :name="order === 'asc' ? 'arrow_upward' : 'arrow_downward'" :size="16" class="arrow" />
  </button>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

/** A table's column heading, which sorts by the column. */
defineProps<{ label: string; active: boolean; order: SortOrder }>()

const emit = defineEmits<{ (e: 'sort'): void }>()
</script>

<style scoped>
.table-sort-button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 100%;
  height: 32px;
  margin-left: -8px;
  padding: 0 8px;
  border-radius: 8px;
  color: var(--schemes-on-surface-variant);
  white-space: nowrap;

  &.active {
    color: var(--schemes-on-surface);
  }

  .arrow {
    color: var(--schemes-primary);
  }
}
</style>
