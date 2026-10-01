<template>
  <div ref="scroller" :class="{ nested }" class="scroll-mask-y virtual-scroller will-change-transform overflow-scroll">
    <div :style="{ height: `${totalHeight}px` }" class="will-change-transform overflow-hidden">
      <div :style="{ transform: `translateY(${offsetY}px)` }" class="will-change-transform items-wrapper">
        <slot v-for="item in renderedItems" :item="item" />
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref, toRefs } from 'vue'
import { useScrollViewport } from '@/composables/useScrollViewport'

const props = defineProps<{ items: any[]; itemHeight: number }>()
const emit = defineEmits<{
  (e: 'scrolled-to-end'): void
}>()

const { items, itemHeight } = toRefs(props)

const scroller = ref<HTMLElement>()
const renderAhead = 5

const {
  scrollTop,
  height: scrollerHeight,
  nested,
  nearEnd,
  scrollTo,
} = useScrollViewport(scroller, () => nearEnd(itemHeight.value) && emit('scrolled-to-end'))

const totalHeight = computed(() => items.value.length * itemHeight.value)
const startPosition = computed(() => Math.max(0, Math.floor(scrollTop.value / itemHeight.value) - renderAhead))
const offsetY = computed(() => startPosition.value * itemHeight.value)

const renderedItems = computed(() => {
  let count = Math.ceil(scrollerHeight.value / itemHeight.value) + 2 * renderAhead
  count = Math.min(items.value.length - startPosition.value, count)
  return items.value.slice(startPosition.value, startPosition.value + count)
})

const scrollToIndex = (index: number) =>
  scrollTo(index * itemHeight.value - scrollerHeight.value / 2 + itemHeight.value / 2)

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
