<template>
  <DropdownMenuRoot v-model:open="open" :modal="false">
    <DropdownMenuTrigger as-child>
      <slot name="anchor" />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent :align :collision-padding="8" :side :side-offset="4" as-child>
        <M3Menu :style="minWidth && { minWidth: `${minWidth}px` }" data-testid="menu-popover">
          <slot />
        </M3Menu>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>

<script lang="ts" setup>
import { DropdownMenuContent, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuTrigger } from 'reka-ui'
import { computed } from 'vue'

import M3Menu from '@/components/m3/M3Menu.vue'

type Side = 'top' | 'right' | 'bottom' | 'left'

/**
 * A menu opened from its anchor (`#anchor`: a button), with M3MenuItems in it.
 * Reka UI runs it: the anchor toggles it, arrow keys move through the items,
 * Escape or a click elsewhere closes it, and focus goes back to the anchor.
 * It lives in a portal, so a scrolling panel around the anchor can't clip it.
 */
const props = withDefaults(
  defineProps<{
    placement?: Side | `${Side}-${'start' | 'end'}`
    /** In px. */
    minWidth?: number
  }>(),
  { placement: 'bottom-end', minWidth: undefined },
)

const open = defineModel<boolean>('open', { default: false })

const side = computed(() => props.placement.split('-')[0] as Side)
const align = computed(() => (props.placement.split('-')[1] ?? 'center') as 'start' | 'end' | 'center')
</script>
