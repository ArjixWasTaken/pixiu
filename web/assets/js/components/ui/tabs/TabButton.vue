<template>
  <button
    :aria-selected="selected"
    class="tab-button m3-state m3-title-small"
    role="tab"
    type="button"
    @click.prevent="emit('click')"
  >
    <span class="inner">
      <slot />
    </span>
  </button>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'

const props = defineProps<{ selected: boolean }>()
const emit = defineEmits<{ (e: 'click'): void }>()

const { selected } = toRefs(props)
</script>

<style scoped>
.tab-button {
  display: flex;
  justify-content: center;
  min-height: 48px;
  padding: 0 16px;
  color: var(--schemes-on-surface-variant);
  white-space: nowrap;
  cursor: pointer;

  &[aria-selected='true'] {
    color: var(--schemes-primary);

    .inner::after {
      content: '';
      position: absolute;
      left: 2px;
      right: 2px;
      bottom: 0;
      height: 3px;
      border-radius: 3px 3px 0 0;
      background: var(--schemes-primary);
    }
  }
}

.inner {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
