<template>
  <nav class="m3-navigation-drawer">
    <template v-for="(entry, index) in items" :key="isHeader(entry) ? `h:${index}` : entry.id">
      <h3 v-if="isHeader(entry)" class="header m3-title-small">{{ entry.header }}</h3>
      <component
        :is="entry.href ? 'a' : 'button'"
        v-else
        :aria-current="entry.id === value ? 'page' : undefined"
        :class="{ active: entry.id === value }"
        :href="entry.href"
        :type="entry.href ? undefined : 'button'"
        class="item m3-state m3-label-large"
        @click="onClick(entry, $event)"
      >
        <M3Icon :class="{ spin: entry.spin }" :fill="entry.id === value" :name="entry.icon" />
        <span class="label">{{ entry.label }}</span>
        <span v-if="entry.badge" class="badge">{{ entry.badge }}</span>
        <M3Icon v-if="entry.trailingIcon" :name="entry.trailingIcon" :size="20" />
      </component>
    </template>
  </nav>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'
import { isHeader } from '@/components/m3/navigation'
import type { M3NavEntry, M3NavItem } from '@/components/m3/navigation'

defineProps<{ items: M3NavEntry[]; value?: string }>()

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
.m3-navigation-drawer {
  display: flex;
  flex-direction: column;
  padding: 0 12px 12px;
}

.header {
  margin: 0;
  padding: 18px 16px;
  color: var(--schemes-on-surface-variant);
}

.item {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 56px;
  flex-shrink: 0;
  padding: 0 24px 0 16px;
  border: 0;
  border-radius: 9999px;
  background: transparent;
  color: var(--schemes-on-surface-variant);
  text-align: start;
  text-decoration: none;
  cursor: pointer;

  &:hover {
    color: var(--schemes-on-surface);
  }

  &.active {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge {
  flex-shrink: 0;
}

.spin {
  animation: m3-spin 2s linear infinite;
}
</style>
