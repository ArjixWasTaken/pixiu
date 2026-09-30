<template>
  <article class="screen-empty-state" data-testid="screen-empty-state" data-vue="ScreenEmptyState">
    <span class="icon">
      <slot name="icon">
        <M3Icon :size="64" name="search" />
      </slot>
    </span>
    <div class="text">
      <slot>Placeholder text goes here.</slot>
    </div>
  </article>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'
</script>

<style lang="postcss" scoped>
.screen-empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px;
  text-align: center;
  color: var(--schemes-on-surface-variant);
}

.icon {
  font-size: 64px;
  line-height: 1;
}

.text {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 48ch;
  color: var(--schemes-on-surface);
  font-size: calc(var(--static-headline-small-size) * 1px);
  line-height: calc(var(--static-headline-small-line-height) * 1px);

  :deep(a) {
    color: var(--schemes-primary);
  }

  :deep(.secondary) {
    color: var(--schemes-on-surface-variant);
    font-size: calc(var(--static-body-large-size) * 1px);
    line-height: calc(var(--static-body-large-line-height) * 1px);
  }
}
</style>
