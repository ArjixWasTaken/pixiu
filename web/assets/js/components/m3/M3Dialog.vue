<template>
  <section :class="{ 'with-icon': icon }" class="m3-dialog" role="dialog" aria-modal="true">
    <M3Icon v-if="icon" :name="icon" class="icon" />
    <h2 v-if="headline || $slots.headline" class="m3-headline-small headline">
      <slot name="headline">{{ headline }}</slot>
    </h2>
    <div class="body m3-body-medium">
      <slot />
    </div>
    <footer v-if="$slots.actions" class="actions">
      <slot name="actions" />
    </footer>
  </section>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

defineProps<{ headline?: string; icon?: string }>()
</script>

<style scoped>
.m3-dialog {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: min(280px, 100%);
  max-width: 100%;
  max-height: calc(100vh - 48px);
  padding: 24px;
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);
  box-shadow: var(--m3-elevation-3);
  box-sizing: border-box;

  &.with-icon {
    align-items: center;
    text-align: center;

    .body {
      text-align: start;
      align-self: stretch;
    }
  }
}

.icon {
  color: var(--schemes-secondary);
}

.headline {
  margin: 0;
  color: var(--schemes-on-surface);
}

.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.actions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
  padding-top: 8px;
}
</style>
