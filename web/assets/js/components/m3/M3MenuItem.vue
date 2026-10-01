<template>
  <DropdownMenuItem
    :as="tag"
    :class="{ selected, disabled }"
    :disabled
    class="m3-menu-item m3-state m3-label-large"
    @select="focusTrigger"
  >
    <slot name="leading">
      <M3Icon v-if="icon" :name="icon" :size="20" class="leading" />
    </slot>
    <span class="label">
      <slot>{{ label }}</slot>
    </span>
    <span v-if="$slots.trailing || trailingText" class="trailing m3-label-large">
      <slot name="trailing">{{ trailingText }}</slot>
    </span>
  </DropdownMenuItem>
</template>

<script lang="ts" setup>
import { DropdownMenuItem, injectDropdownMenuRootContext } from 'reka-ui'
import M3Icon from '@/components/m3/M3Icon.vue'

/** An item of an M3MenuPopover. Choosing it (a click, Enter) runs its `@click` and closes the menu. */
withDefaults(
  defineProps<{
    label?: string
    icon?: string
    trailingText?: string
    selected?: boolean
    disabled?: boolean
    tag?: string
  }>(),
  {
    selected: false,
    disabled: false,
    tag: 'li',
  },
)

const menu = injectDropdownMenuRootContext()

/** Focus is back on the menu's button before the item does its part: a dialog it opens gives it back there too. */
const focusTrigger = () => menu.triggerElement.value?.focus({ preventScroll: true })
</script>

<style scoped>
.m3-menu-item {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: var(--m3-menu-item-height);
  padding: 0 12px;
  color: var(--schemes-on-surface);
  cursor: pointer;
  list-style: none;
  white-space: nowrap;
  outline: none;

  &.selected {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }

  &.disabled {
    opacity: 0.38;
    pointer-events: none;
  }
}

.leading,
.trailing {
  color: var(--schemes-on-surface-variant);
  display: flex;
  align-items: center;
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
