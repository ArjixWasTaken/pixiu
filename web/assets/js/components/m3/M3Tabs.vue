<template>
  <div
    ref="list"
    :class="{ secondary, sticky, scrollable, 'fade-start': overflowsStart, 'fade-end': overflowsEnd }"
    class="m3-tabs"
    role="tablist"
    @scroll.passive="measure"
  >
    <template v-for="(tab, index) in tabs" :key="tab.id">
      <span v-if="startsGroup(index)" class="group m3-label-medium" role="presentation">{{ tab.group }}</span>
      <button
        :id="idPrefix && `${idPrefix}-tab-${tab.id}`"
        :aria-controls="idPrefix && `${idPrefix}-panel-${tab.id}`"
        :aria-selected="tab.id === value"
        :class="{ active: tab.id === value, 'with-icon': tab.icon && !secondary }"
        :data-testid="idPrefix && `${idPrefix}-tab-${tab.id}`"
        class="tab m3-state m3-title-small"
        role="tab"
        type="button"
        @click="value = tab.id"
      >
        <span class="inner">
          <M3Icon v-if="tab.icon" :fill="tab.id === value" :name="tab.icon" />
          <span>{{ tab.label }}</span>
          <span v-if="tab.id === value" class="indicator" />
        </span>
      </button>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'
import { useResizeObserver } from '@vueuse/core'
import M3Icon from '@/components/m3/M3Icon.vue'

export interface M3Tab {
  id: string
  label: string
  icon?: string
  /** Tabs of a group follow its name, set apart from those before. */
  group?: string
}

const props = withDefaults(
  defineProps<{
    tabs: M3Tab[]
    secondary?: boolean
    /** Stays at the top while what is under it scrolls. */
    sticky?: boolean
    /** Tabs keep their own width, from the start, and scroll when they run out of room. */
    scrollable?: boolean
    /** Names the tabs `<prefix>-tab-<id>`, each controlling the panel `<prefix>-panel-<id>`. */
    idPrefix?: string
  }>(),
  { secondary: false, sticky: false, scrollable: false, idPrefix: undefined },
)

const value = defineModel<string>()

const startsGroup = (index: number) =>
  Boolean(props.tabs[index].group) && props.tabs[index].group !== props.tabs[index - 1]?.group

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
const revealSelected = async () => {
  await nextTick()
  list.value?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView?.({
    block: 'nearest',
    inline: 'nearest',
  })
  measure()
}

useResizeObserver(list, measure)
onMounted(revealSelected)
watch(value, revealSelected)
</script>

<style scoped>
.m3-tabs {
  display: flex;
  overflow-x: auto;
  scrollbar-width: none;

  &.sticky {
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--schemes-surface);
  }

  &.scrollable .tab {
    flex: 0 0 auto;
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

.tab {
  flex: 1 0 auto;
  display: flex;
  justify-content: center;
  height: var(--m3-tab-height);
  padding: 0 16px;
  border: 0;
  background: transparent;
  color: var(--schemes-on-surface-variant);
  cursor: pointer;

  &.with-icon {
    height: var(--m3-tab-height-icon);
  }

  &.active {
    color: var(--schemes-primary);
  }
}

.inner {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 100%;
}

.inner .m3-icon {
  display: var(--m3-tab-icon);
}

.with-icon .inner {
  flex-direction: column;
  justify-content: center;
  gap: 2px;
}

.group {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  margin: 10px 0 10px 16px;
  padding: 0 4px 0 20px;
  border-left: 1px solid var(--schemes-outline-variant);
  color: var(--schemes-outline);
  font-size: calc(var(--static-label-small-size) * 1px);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  white-space: nowrap;
}

.indicator {
  position: absolute;
  left: 2px;
  right: 2px;
  bottom: 0;
  height: 3px;
  border-radius: 3px 3px 0 0;
  background: var(--schemes-primary);
}

.secondary {
  .tab {
    position: relative;

    &.active {
      color: var(--schemes-on-surface);
    }
  }

  .inner {
    position: static;
  }

  .indicator {
    left: 0;
    right: 0;
    height: 2px;
    border-radius: 0;
  }
}
</style>
