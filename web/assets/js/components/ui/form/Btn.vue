<template>
  <button
    v-if="tag === 'button'"
    ref="button"
    :data-variant="variant"
    :data-size="size"
    class="k-btn m3-state m3-label-large"
    type="button"
  >
    <slot>Click me</slot>
  </button>
  <a v-else ref="button" :data-variant="variant" :data-size="size" class="k-btn m3-state m3-label-large">
    <slot>Click me</slot>
  </a>
</template>

<script lang="ts" setup>
import { ref } from 'vue'

type Variant = 'success' | 'destructive' | 'highlight' | 'ghost'
type Size = 'small' | 'large'

withDefaults(defineProps<{ tag?: 'button' | 'a'; variant?: Variant; size?: Size }>(), {
  tag: 'button',
})

const button = ref<HTMLButtonElement | HTMLAnchorElement>()

defineExpose({
  button,
})
</script>

<style lang="postcss" scoped>
/* koel's button, drawn as a Material 3 button: filled by default. */
.k-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 40px;
  padding: 0 24px;
  border: 1px solid transparent;
  border-radius: 9999px;
  background: var(--schemes-primary);
  color: var(--schemes-on-primary);
  cursor: pointer;
  white-space: nowrap;
  text-decoration: none;

  &:not([disabled]):hover {
    box-shadow: var(--m3-elevation-1);
  }

  &[disabled] {
    cursor: default;
    box-shadow: none;
    background: color-mix(in srgb, var(--schemes-on-surface) 10%, transparent);
    color: color-mix(in srgb, var(--schemes-on-surface) 38%, transparent);
  }

  &[data-size='large'] {
    min-height: 56px;
  }

  &[data-size='small'] {
    min-height: 32px;
    padding: 0 16px;
  }

  &[data-variant='success'] {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }

  &[data-variant='destructive'] {
    background: var(--schemes-error);
    color: var(--schemes-on-error);
  }

  &[data-variant='ghost'] {
    padding: 0 12px;
    background: transparent;
    color: var(--schemes-primary);

    &:not([disabled]):hover {
      box-shadow: none;
    }
  }

  &[unrounded] {
    border-radius: 0;
  }

  &[bordered] {
    padding: 0 24px;
    border-color: var(--schemes-outline-variant);
    background: transparent;
    color: var(--schemes-on-surface-variant);

    &[data-variant='destructive'] {
      border-color: var(--schemes-error);
      color: var(--schemes-error);
    }

    &[data-variant='highlight'],
    &[data-variant='success'] {
      color: var(--schemes-primary);
    }
  }
}
</style>
