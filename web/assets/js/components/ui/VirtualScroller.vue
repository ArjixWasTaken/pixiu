<template>
  <div ref="list" :class="{ nested }" class="scroll-mask-y virtual-scroller overflow-scroll">
    <div :style="{ height: `${virtualizer.getTotalSize()}px` }" class="relative">
      <div :style="{ transform: `translateY(${offsetY}px)` }" class="will-change-transform items-wrapper">
        <div v-for="row in rows" :key="row.index" :ref="measure" :data-index="row.index">
          <slot :item="items[row.index]" />
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { useVirtualizer } from '@tanstack/vue-virtual'
import { computed, toRefs, watch } from 'vue'
import { useScrollContainer } from '@/composables/useScrollContainer'

/**
 * A long list, of which only the rows in view (and a few around) are rendered.
 * `itemHeight` is the estimate; each row's real height is measured.
 */
const props = defineProps<{ items: any[]; itemHeight: number }>()
const emit = defineEmits<{ (e: 'scrolled-to-end'): void }>()

const { items, itemHeight } = toRefs(props)

const { list, scroller, nested, margin } = useScrollContainer()

const virtualizer = useVirtualizer(
  computed(() => ({
    count: items.value.length,
    getScrollElement: () => scroller.value,
    estimateSize: () => itemHeight.value,
    overscan: 5,
    scrollMargin: margin.value,
  })),
)

const rows = computed(() => virtualizer.value.getVirtualItems())
const offsetY = computed(() => (rows.value[0]?.start ?? margin.value) - margin.value)

const measure = (el: unknown) => el instanceof Element && virtualizer.value.measureElement(el)

/** The last rows are rendered: the end is near. */
const nearEnd = computed(() => items.value.length > 0 && rows.value.at(-1)?.index === items.value.length - 1)

watch(nearEnd, near => near && emit('scrolled-to-end'))

const scrollToIndex = (index: number) => virtualizer.value.scrollToIndex(index, { align: 'center', behavior: 'smooth' })

defineExpose({ scrollToIndex })
</script>

<style lang="postcss" scoped>
/* On a screen, the screen scrolls the list. */
.virtual-scroller.nested {
  overflow: visible;
  mask-image: none;
}

.virtual-scroller:not(.nested) {
  @supports (scrollbar-gutter: stable) {
    overflow: auto;
    scrollbar-gutter: stable;

    @media (hover: none) {
      scrollbar-gutter: auto;
    }
  }
}
</style>
