<template>
  <PopoverRoot v-model:open="open">
    <PopoverTrigger as-child>
      <slot name="anchor" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent :align :collision-padding="8" :side :side-offset="gap" v-bind="$attrs">
        <slot />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<script lang="ts" setup>
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import { computed } from 'vue'

type Side = 'top' | 'right' | 'bottom' | 'left'

/**
 * A panel opened from its anchor (`#anchor`: a button), beside it. Reka UI runs
 * it: the anchor toggles it, Escape or a click elsewhere closes it, and focus
 * goes back to the anchor. Classes and attributes go to the panel.
 */
defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    placement?: Side | `${Side}-${'start' | 'end'}`
    /** Pixel gap between anchor and panel. */
    gap?: number
  }>(),
  { placement: 'bottom', gap: 6 },
)

const open = defineModel<boolean>('open', { default: false })

const side = computed(() => props.placement.split('-')[0] as Side)
const align = computed(() => (props.placement.split('-')[1] ?? 'center') as 'start' | 'end' | 'center')
</script>
