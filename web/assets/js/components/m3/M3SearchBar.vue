<template>
  <form
    :class="rootAttrs.class"
    :style="rootAttrs.style"
    class="m3-search-bar"
    role="search"
    @submit.prevent="emit('submit', value)"
  >
    <slot name="leading">
      <M3Icon class="icon" name="search" />
    </slot>
    <input
      ref="input"
      v-model="value"
      v-bind="inputAttrs"
      :aria-label="placeholder"
      :placeholder
      class="m3-body-large"
      type="search"
    />
    <slot name="trailing" />
  </form>
</template>

<script lang="ts" setup>
import { useTemplateRef } from 'vue'
import { useSplitAttrs } from '@/components/m3/useSplitAttrs'
import M3Icon from '@/components/m3/M3Icon.vue'

defineOptions({ inheritAttrs: false })

defineProps<{ placeholder?: string }>()

const emit = defineEmits<{ (e: 'submit', value: string): void }>()

const value = defineModel<string>({ default: '' })

const { rootAttrs, inputAttrs } = useSplitAttrs()

const input = useTemplateRef('input')

defineExpose({ focus: () => input.value?.focus() })
</script>

<style scoped>
.m3-search-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  height: var(--m3-field-height);
  padding: 0 16px;
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface);
  min-width: 0;
  transition: background 150ms linear;

  /* Typing goes here: the bar lights up, and keyboard focus gets a ring too. */
  &:focus-within {
    background: var(--schemes-surface-container-highest);
  }

  &:has(input:focus-visible) {
    outline: 2px solid var(--schemes-secondary);
    outline-offset: 2px;
  }
}

.icon {
  color: var(--schemes-on-surface);
}

input {
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--schemes-on-surface);

  &::placeholder {
    color: var(--schemes-on-surface-variant);
  }

  &::-webkit-search-cancel-button {
    display: none;
  }
}
</style>
