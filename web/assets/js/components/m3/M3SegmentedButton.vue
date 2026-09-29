<template>
  <div class="m3-segmented" role="group">
    <button
      v-for="segment in segments"
      :key="segment.id"
      :aria-label="segment.label || segment.icon"
      :aria-pressed="segment.id === value"
      :class="{ selected: segment.id === value }"
      :title="segment.label || undefined"
      class="segment m3-state m3-label-large"
      type="button"
      @click="value = segment.id"
    >
      <M3Icon v-if="segment.id === value && (segment.label || !segment.icon)" :size="18" name="check" />
      <M3Icon v-else-if="segment.icon" :size="18" :name="segment.icon" />
      <span v-if="segment.label">{{ segment.label }}</span>
    </button>
  </div>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

export interface M3Segment {
  id: string
  label?: string
  icon?: string
}

defineProps<{ segments: M3Segment[] }>()

const value = defineModel<string>()
</script>

<style scoped>
.m3-segmented {
  display: inline-flex;
  height: 40px;
  flex-shrink: 0;
}

.segment {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-width: 48px;
  padding: 0 12px;
  border: 1px solid var(--schemes-outline);
  background: transparent;
  color: var(--schemes-on-surface);
  cursor: pointer;
  white-space: nowrap;

  & + & {
    margin-left: -1px;
  }

  &:first-child {
    border-radius: 20px 0 0 20px;
  }

  &:last-child {
    border-radius: 0 20px 20px 0;
  }

  &.selected {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}
</style>
