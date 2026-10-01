<template>
  <input v-model="value" :checked="value" class="m3-switch" role="switch" type="checkbox" />
</template>

<script lang="ts" setup>
const value = defineModel<boolean>()
</script>

<style scoped>
.m3-switch {
  position: relative;
  display: inline-block;
  flex-shrink: 0;
  width: 52px;
  height: 32px;
  vertical-align: middle;
  border: 2px solid var(--schemes-outline);
  border-radius: 16px;
  background: var(--schemes-surface-container-highest);
  cursor: pointer;
  appearance: none;
  transition: background-color 150ms linear;

  &::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 6px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--schemes-outline);
    transform: translateY(-50%);
    transition:
      left 200ms var(--m3-ease),
      width 200ms var(--m3-ease),
      height 200ms var(--m3-ease);
  }

  &:checked {
    border-color: var(--schemes-primary);
    background: var(--schemes-primary);

    &::after {
      left: 22px;
      width: 24px;
      height: 24px;
      background: var(--schemes-on-primary);
    }
  }

  &:focus-visible {
    outline: 2px solid var(--schemes-secondary);
    outline-offset: 2px;
  }

  &:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }
}
</style>
