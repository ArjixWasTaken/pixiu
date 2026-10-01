<template>
  <DropdownMenuSub v-if="$slots.subMenuItems && !isMobile">
    <DropdownMenuSubTrigger as="li" class="has-sub">
      <span v-if="$slots.icon" class="menu-icon"><slot name="icon" /></span>
      <span class="label"><slot /></span>
      <M3Icon class="sub-arrow" name="arrow_right" />
    </DropdownMenuSubTrigger>
    <DropdownMenuPortal>
      <DropdownMenuSubContent :align-offset="-8" :collision-padding="8" class="menu context-menu submenu">
        <ul role="none">
          <slot name="subMenuItems" />
        </ul>
      </DropdownMenuSubContent>
    </DropdownMenuPortal>
  </DropdownMenuSub>

  <template v-else-if="$slots.subMenuItems">
    <DropdownMenuItem :aria-expanded="expanded" as="li" class="has-sub" @select.prevent="expanded = !expanded">
      <span v-if="$slots.icon" class="menu-icon"><slot name="icon" /></span>
      <span class="label"><slot /></span>
      <M3Icon :name="expanded ? 'expand_less' : 'expand_more'" class="sub-arrow" />
    </DropdownMenuItem>
    <ul v-if="expanded" class="submenu in-place" role="none">
      <slot name="subMenuItems" />
    </ul>
  </template>

  <DropdownMenuItem v-else as="li" @select="choose">
    <span v-if="$slots.icon" class="menu-icon"><slot name="icon" /></span>
    <span class="label"><slot /></span>
  </DropdownMenuItem>
</template>

<script setup lang="ts">
import {
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from 'reka-ui'
import { inject, ref } from 'vue'
import { ContextMenuOpenerKey } from '@/config/symbols'
import { useViewport } from '@/composables/useViewport'

import M3Icon from '@/components/m3/M3Icon.vue'

/**
 * An item of a context menu. With `subMenuItems`, it opens a submenu: beside
 * it with a mouse, in place under it in the phone's sheet.
 */
const emit = defineEmits<{ (e: 'click'): void }>()

const { isMobile } = useViewport()
const expanded = ref(false)

const focusOpener = inject(ContextMenuOpenerKey, () => {})

const choose = () => {
  focusOpener()
  emit('click')
}
</script>

<style scoped>
.menu-icon {
  display: flex;
  justify-content: center;
  width: 20px;
  color: var(--schemes-on-surface-variant);
}

.label {
  flex: 1;
  min-width: 0;
  max-width: 14rem;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sub-arrow {
  margin-left: auto;
  color: var(--schemes-on-surface-variant);
}
</style>
