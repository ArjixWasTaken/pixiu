<template>
  <div ref="anchor" class="m3-menu-anchor">
    <slot name="anchor" />
    <!-- The menu lives in the top layer (a popover), so a scrolling panel
         around its anchor cannot clip it. -->
    <div ref="popover" class="m3-menu-popover" popover="manual">
      <M3Menu v-if="open" :class="menuClass" data-testid="menu-popover">
        <slot />
      </M3Menu>
    </div>
  </div>
</template>

<script lang="ts" setup>
import type { Placement } from '@floating-ui/dom'
import { computePosition, flip, shift, size } from '@floating-ui/dom'
import { nextTick, onBeforeUnmount, useTemplateRef, watch } from 'vue'
import { useEventListener } from '@vueuse/core'
import { logger } from '@/utils/logger'

import M3Menu from '@/components/m3/M3Menu.vue'

const props = withDefaults(defineProps<{ placement?: Placement; menuClass?: string }>(), {
  placement: 'bottom-end',
  menuClass: '',
})

const open = defineModel<boolean>('open', { default: false })

const anchor = useTemplateRef('anchor')
const popover = useTemplateRef('popover')

const supportsPopover = typeof HTMLElement !== 'undefined' && 'showPopover' in HTMLElement.prototype

const place = async () => {
  const menu = popover.value?.firstElementChild as HTMLElement | null
  if (!anchor.value || !popover.value || !menu) {
    return
  }

  const { x, y } = await computePosition(anchor.value, popover.value, {
    placement: props.placement,
    strategy: 'fixed',
    middleware: [
      flip(),
      shift({ padding: 8 }),
      // Taller than the room it has: it scrolls.
      size({
        padding: 8,
        apply: ({ availableHeight }) => {
          menu.style.maxHeight = `${Math.max(120, availableHeight)}px`
        },
      }),
    ],
  })
  // Closed or gone while it was measured.
  if (!popover.value) {
    return
  }
  popover.value.style.left = `${x}px`
  popover.value.style.top = `${y}px`
}

/** Whether the popover is shown (tracked here: `:popover-open` is newer still). */
let shown = false

const show = async () => {
  if (!popover.value) {
    return
  }

  if (supportsPopover && !shown) {
    popover.value.showPopover()
    shown = true
  }

  await nextTick()

  try {
    await place()
  } catch (error: unknown) {
    logger.error(error)
  }
}

const hide = () => {
  if (supportsPopover && shown) {
    popover.value?.hidePopover()
    shown = false
  }
}

watch(open, isOpen => (isOpen ? show() : hide()), { flush: 'post' })

// A press anywhere else closes it; presses on the anchor (its button, the
// menu itself) are the anchor's to handle.
useEventListener(document, 'pointerdown', (event: PointerEvent) => {
  if (open.value && event.target instanceof Node && !anchor.value?.contains(event.target)) {
    open.value = false
  }
})

useEventListener(document, 'keydown', (event: KeyboardEvent) => {
  if (open.value && event.key === 'Escape') {
    open.value = false
  }
})

// Scrolling or resizing moves the anchor: follow it.
useEventListener(window, 'resize', () => open.value && place())
useEventListener(document, 'scroll', () => open.value && place(), { capture: true, passive: true })

onBeforeUnmount(hide)
</script>

<style scoped>
.m3-menu-anchor {
  display: inline-flex;
}

.m3-menu-popover {
  position: fixed;
  inset: auto;
  z-index: 1000;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  overflow: visible;

  :deep(.m3-menu) {
    overflow-y: auto;
  }
}
</style>
