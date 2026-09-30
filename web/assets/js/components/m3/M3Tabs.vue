<template>
  <div :class="{ secondary }" class="m3-tabs" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      :aria-selected="tab.id === value"
      :class="{ active: tab.id === value, 'with-icon': tab.icon && !secondary }"
      class="tab m3-state m3-title-small"
      role="tab"
      type="button"
      @click="value = tab.id"
    >
      <span class="inner">
        <M3Icon v-if="tab.icon" :fill="tab.id === value" :name="tab.icon" />
        <span>{{ tab.label }}</span>
        <span v-if="tab.id === value" class="indicator" />
      </span>
    </button>
  </div>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

export interface M3Tab {
  id: string
  label: string
  icon?: string
}

withDefaults(defineProps<{ tabs: M3Tab[]; secondary?: boolean }>(), { secondary: false })

const value = defineModel<string>()
</script>

<style scoped>
.m3-tabs {
  display: flex;
  overflow-x: auto;
  scrollbar-width: none;
}

.tab {
  flex: 1 0 auto;
  display: flex;
  justify-content: center;
  height: 48px;
  padding: 0 16px;
  border: 0;
  background: transparent;
  color: var(--schemes-on-surface-variant);
  cursor: pointer;

  &.with-icon {
    height: 64px;
  }

  &.active {
    color: var(--schemes-primary);
  }
}

.inner {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 100%;
}

.with-icon .inner {
  flex-direction: column;
  justify-content: center;
  gap: 2px;
}

.indicator {
  position: absolute;
  left: 2px;
  right: 2px;
  bottom: 0;
  height: 3px;
  border-radius: 3px 3px 0 0;
  background: var(--schemes-primary);
}

.secondary {
  .tab {
    position: relative;

    &.active {
      color: var(--schemes-on-surface);
    }
  }

  .inner {
    position: static;
  }

  .indicator {
    left: 0;
    right: 0;
    height: 2px;
    border-radius: 0;
  }
}
</style>
