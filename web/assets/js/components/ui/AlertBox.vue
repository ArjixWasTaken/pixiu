<template>
  <div :class="`alert-box-${props.type}`" class="alert-box">
    <M3Icon :name="icon" fill />
    <div class="flex-1 min-w-0">
      <slot />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'

import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(defineProps<{ type?: 'default' | 'info' | 'danger' | 'success' | 'warning' }>(), {
  type: 'default',
})

const icon = computed(
  () => ({ default: 'info', info: 'info', danger: 'error', success: 'check_circle', warning: 'warning' })[props.type],
)
</script>

<style scoped>
/* A status banner: tonal containers by kind. */
.alert-box {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 12px 16px;
  border-radius: 16px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface);

  &-success {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }

  &-warning {
    background: var(--schemes-tertiary-container);
    color: var(--schemes-on-tertiary-container);
  }

  &-danger {
    background: var(--schemes-error-container);
    color: var(--schemes-on-error-container);
  }
}
</style>
