<template>
  <DropdownMenuCheckboxItem v-model="checked" :disabled class="m3-menu-item m3-state m3-label-large" @select.prevent>
    <M3Icon :fill="checked" :name="checked ? 'check_box' : 'check_box_outline_blank'" :size="20" class="leading" />
    <span class="label">
      <slot>{{ label }}</slot>
    </span>
  </DropdownMenuCheckboxItem>
</template>

<script lang="ts" setup>
import { DropdownMenuCheckboxItem } from 'reka-ui'
import M3Icon from '@/components/m3/M3Icon.vue'

/** An on/off item of an M3MenuPopover: choosing it toggles it, and the menu stays open for the next. */
withDefaults(defineProps<{ label?: string; disabled?: boolean }>(), { disabled: false })

const checked = defineModel<boolean>({ default: false })
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
  white-space: nowrap;
  outline: none;

  &[data-highlighted]::before {
    opacity: 0.1;
  }

  &[data-disabled] {
    cursor: default;
    opacity: 0.38;
  }

  .leading {
    color: var(--schemes-on-surface-variant);
  }

  &[data-state='checked'] .leading {
    color: var(--schemes-primary);
  }
}
</style>
