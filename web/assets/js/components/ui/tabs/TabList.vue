<template>
  <nav
    ref="list"
    :class="{ sticky, 'fade-start': overflowsStart, 'fade-end': overflowsEnd }"
    class="tab-list flex overflow-x-auto px-6 min-h-[48px]"
    role="tablist"
    @scroll.passive="measure"
  >
    <slot />
  </nav>
</template>

<script lang="ts" setup>
import { onMounted, onUpdated, ref, useTemplateRef } from 'vue'
import { useResizeObserver } from '@vueuse/core'

/** `sticky`: stays at the top while the screen scrolls under it. */
withDefaults(defineProps<{ sticky?: boolean }>(), { sticky: false })

const list = useTemplateRef('list')
const overflowsStart = ref(false)
const overflowsEnd = ref(false)

/** Tabs cut off at an edge fade out there, so it shows there are more. */
const measure = () => {
  const el = list.value
  if (!el) {
    return
  }
  overflowsStart.value = el.scrollLeft > 1
  overflowsEnd.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1
}

/** The selected tab scrolls into view, so it is never hidden past an edge. */
const revealSelected = () => {
  const selected = list.value?.querySelector<HTMLElement>('[aria-selected="true"]')
  selected?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' })
  measure()
}

useResizeObserver(list, measure)
onMounted(revealSelected)
onUpdated(revealSelected)
</script>

<style scoped>
.tab-list {
  scrollbar-width: none;
  border-bottom: 1px solid var(--schemes-outline-variant);

  &.sticky {
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--schemes-surface);
  }

  &.fade-start {
    mask-image: linear-gradient(to right, transparent, black 32px);
  }

  &.fade-end {
    mask-image: linear-gradient(to left, transparent, black 32px);
  }

  &.fade-start.fade-end {
    mask-image: linear-gradient(to right, transparent, black 32px, black calc(100% - 32px), transparent);
  }
}
</style>
