<template>
  <button
    :aria-pressed="variant === 'filter' ? selected : undefined"
    :class="[variant, { selected }]"
    class="m3-chip m3-state m3-label-large"
    type="button"
  >
    <M3Icon v-if="variant === 'filter' && selected" :size="18" name="check" />
    <slot v-else name="icon">
      <M3Icon v-if="icon" :name="icon" :size="18" class="leading" />
    </slot>
    <span><slot /></span>
    <slot name="trailing" />
  </button>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

withDefaults(defineProps<{ variant?: 'assist' | 'filter'; icon?: string; selected?: boolean }>(), {
  variant: 'assist',
  selected: false,
})
</script>

<style scoped>
.m3-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 16px;
  border-radius: 8px;
  border: 1px solid var(--schemes-outline-variant);
  background: transparent;
  color: var(--schemes-on-surface-variant);
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;

  &:has(.m3-icon) {
    padding-left: 8px;
  }

  .leading {
    color: var(--schemes-primary);
  }

  &.assist {
    color: var(--schemes-on-surface);
  }

  &.selected {
    border-color: transparent;
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}
</style>
