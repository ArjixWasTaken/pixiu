<template>
  <nav class="m3-navigation-bar">
    <component
      :is="item.href ? 'a' : 'button'"
      v-for="item in items"
      :key="item.id"
      :aria-current="item.id === value ? 'page' : undefined"
      :class="{ active: item.id === value }"
      :href="item.href"
      :type="item.href ? undefined : 'button'"
      class="item"
      @click="onClick(item, $event)"
    >
      <span class="indicator m3-state">
        <M3Icon :fill="item.id === value" :name="item.icon" />
      </span>
      <span class="m3-label-medium">{{ item.label }}</span>
    </component>
  </nav>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'
import type { M3NavItem } from '@/components/m3/navigation'

defineProps<{ items: M3NavItem[]; value?: string }>()

const emit = defineEmits<{ (e: 'select', item: M3NavItem): void }>()

/** Links still work as links (open in a new tab, copy); a plain click is the parent's to handle. */
const onClick = (item: M3NavItem, event: MouseEvent) => {
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) {
    return
  }

  event.preventDefault()
  emit('select', item)
}
</script>

<style scoped>
.m3-navigation-bar {
  display: flex;
  height: 80px;
  background: var(--schemes-surface-container);
  flex-shrink: 0;
}

.item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--schemes-on-surface-variant);
  text-decoration: none;
  cursor: pointer;

  &.active {
    color: var(--schemes-on-surface);

    .indicator {
      background: var(--schemes-secondary-container);
      color: var(--schemes-on-secondary-container);
    }
  }
}

.indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 32px;
  border-radius: 16px;
}
</style>
