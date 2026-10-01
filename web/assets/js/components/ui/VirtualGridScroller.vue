<template>
  <div ref="list" :class="{ nested }" class="scroll-mask-y virtual-grid-scroller overflow-scroll h-full">
    <!-- Measuring phase: an item, as wide as in the grid, for its height, the gaps and the padding.
         A few are offered, in case the first shows nothing. -->
    <div v-if="measuring && items.length" ref="measureContainer" v-bind="$attrs" class="grid measuring">
      <slot v-for="item in items.slice(0, 5)" :item />
    </div>

    <template v-else>
      <div class="height-container will-change-transform overflow-hidden">
        <div ref="gridContainer" v-bind="$attrs" class="grid-container will-change-transform grid">
          <slot v-for="item in renderedItems" :item />
        </div>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { useVirtualizer } from '@tanstack/vue-virtual'
import { useResizeObserver } from '@vueuse/core'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRefs, watch } from 'vue'
import { useScrollContainer } from '@/composables/useScrollContainer'

/**
 * A long grid of cards, as many to a row as fit `minItemWidth`, of which only
 * the rows in view (and a few around) are rendered. One card is measured first
 * for the height of a row.
 */
defineOptions({ inheritAttrs: false })

const props = defineProps<{
  items: any[]
  minItemWidth: number
}>()

const emit = defineEmits<{ (e: 'scrolled-to-end'): void }>()
const { items, minItemWidth } = toRefs(props)

const { scroller, nested, margin, width } = useScrollContainer()

const measureContainer = ref<HTMLElement>()
const gridContainer = ref<HTMLElement>()
const measuredItemHeight = ref(0)
const measuredRowGap = ref(0)
const measuredColumnGap = ref(0)
const measuredPaddingX = ref(0)
const measuredPaddingY = ref(0)
const measuring = ref(true)

const columnCount = computed(() => {
  const contentWidth = width.value - measuredPaddingX.value
  const g = measuredColumnGap.value
  return Math.max(1, Math.floor((contentWidth + g) / (minItemWidth.value + g)))
})

const rowCount = computed(() => Math.ceil(items.value.length / columnCount.value))
const rowHeight = computed(() => measuredItemHeight.value + measuredRowGap.value)

const virtualizer = useVirtualizer(
  computed(() => ({
    count: measuring.value ? 0 : rowCount.value,
    getScrollElement: () => scroller.value,
    estimateSize: () => rowHeight.value,
    overscan: 3,
    scrollMargin: margin.value,
  })),
)

const rows = computed(() => virtualizer.value.getVirtualItems())
const startRow = computed(() => rows.value[0]?.index ?? 0)
const endRow = computed(() => (rows.value.at(-1)?.index ?? -1) + 1)

const renderedItems = computed(() =>
  measuredItemHeight.value
    ? items.value.slice(startRow.value * columnCount.value, endRow.value * columnCount.value)
    : [],
)

const totalHeight = computed(() =>
  rowCount.value ? rowCount.value * rowHeight.value - measuredRowGap.value + measuredPaddingY.value : 0,
)

const cssHeight = computed(() => `${totalHeight.value}px`)
const cssTransform = computed(() => `translateY(${startRow.value * rowHeight.value}px)`)
const cssColumns = computed(() => `repeat(${columnCount.value}, minmax(0, 1fr))`)
/** As many columns as fit, as `columnCount` works them out. */
const cssMeasuringColumns = computed(() => `repeat(auto-fill, minmax(${minItemWidth.value}px, 1fr))`)

const measure = async () => {
  if (!items.value.length) {
    measuring.value = false
    return
  }

  measuring.value = true
  await nextTick()

  if (!measureContainer.value) {
    measuring.value = false
    return
  }

  const style = getComputedStyle(measureContainer.value)
  measuredRowGap.value = parseFloat(style.rowGap) || parseFloat(style.gap) || 0
  measuredColumnGap.value = parseFloat(style.columnGap) || parseFloat(style.gap) || 0
  measuredPaddingX.value = (parseFloat(style.paddingLeft) || 0) + (parseFloat(style.paddingRight) || 0)
  measuredPaddingY.value = (parseFloat(style.paddingTop) || 0) + (parseFloat(style.paddingBottom) || 0)

  const firstChild = measureContainer.value.firstElementChild as HTMLElement | null

  if (firstChild) {
    measuredItemHeight.value = firstChild.offsetHeight
  }

  measuring.value = false
}

onMounted(measure)

watch(minItemWidth, () => measure())

// Cards are as tall as they are wide (their covers are square): as the grid's width
// changes, a row's height is read again off the rows rendered. On the next frame:
// resizing the grid within the observer's own round is a loop.
let frame = 0
onBeforeUnmount(() => cancelAnimationFrame(frame))

useResizeObserver(gridContainer, () => {
  cancelAnimationFrame(frame)
  frame = requestAnimationFrame(() => {
    const rendered = endRow.value - startRow.value
    const grid = gridContainer.value

    if (!grid || rendered < 1) {
      return
    }

    const height = (grid.offsetHeight - measuredPaddingY.value - (rendered - 1) * measuredRowGap.value) / rendered
    height > 0 && Math.abs(height - measuredItemHeight.value) > 0.5 && (measuredItemHeight.value = height)
  })
})

watch(
  () => items.value.length,
  async (newLen: number, oldLen: number) => {
    if (oldLen === 0 && newLen > 0 && !measuredItemHeight.value) {
      await measure()
    }
  },
)

/** The last rows are rendered: the end is near. */
const nearEnd = computed(() => rowCount.value > 0 && endRow.value === rowCount.value)

watch(nearEnd, near => near && emit('scrolled-to-end'))

const scrollToTop = () => virtualizer.value.scrollToOffset(margin.value, { behavior: 'smooth' })

defineExpose({ scrollToTop })
</script>

<style lang="postcss" scoped>
/* On a screen, the screen scrolls the grid. */
.virtual-grid-scroller.nested {
  overflow: visible;
  height: auto;
  mask-image: none;
}

.virtual-grid-scroller:not(.nested) {
  @supports (scrollbar-gutter: stable) {
    overflow: auto;
    scrollbar-gutter: stable;

    @media (hover: none) {
      scrollbar-gutter: auto;
    }
  }
}

.height-container {
  height: v-bind(cssHeight);
}

.grid-container {
  transform: v-bind(cssTransform);
  grid-template-columns: v-bind(cssColumns);
}

.measuring {
  grid-template-columns: v-bind(cssMeasuringColumns);
}
</style>
