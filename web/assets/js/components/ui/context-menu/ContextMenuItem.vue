<template>
  <li
    ref="el"
    :class="cssClasses"
    class="focus:outline-hidden"
    tabindex="-1"
    @mouseover="focus()"
    @click.prevent="emit('click')"
  >
    <span v-if="hasIconSlot" class="menu-icon flex w-5 justify-center text-(--schemes-on-surface-variant)">
      <slot name="icon" />
    </span>

    <span class="label flex-1 min-w-0 max-w-56 truncate">
      <slot />
    </span>

    <ul v-if="hasSubMenuItems" class="context-menu submenu" tabindex="-1">
      <slot name="subMenuItems" />
    </ul>

    <M3Icon v-if="hasSubMenuItems" class="sub-arrow ml-auto text-(--schemes-on-surface-variant)" name="arrow_right" />
  </li>
</template>

<script setup lang="ts">
import { ref, useSlots } from 'vue'

import M3Icon from '@/components/m3/M3Icon.vue'

const emit = defineEmits<{ (e: 'click'): void }>()

const el = ref<HTMLLIElement>()

const focus = async () => el.value?.focus()

const slots = useSlots()

const hasIconSlot = Boolean(slots.icon)
const hasSubMenuItems = Boolean(slots.subMenuItems)

let cssClasses = hasIconSlot ? 'flex items-center gap-3' : ''

if (hasSubMenuItems) {
  cssClasses += ' has-sub'
}
</script>
