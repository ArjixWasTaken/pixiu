<template>
  <div class="w-full min-w-0">
    <Teleport v-if="actionsHost && hasOverflow" :to="actionsHost">
      <M3IconButton
        v-for="ctrl in controls"
        :key="ctrl.title"
        :icon="ctrl.icon"
        :label="ctrl.title"
        @click="slide(ctrl.direction)"
      />
    </Teleport>

    <nav v-else-if="hasOverflow" class="flex justify-end gap-1 mb-2">
      <M3IconButton
        v-for="ctrl in controls"
        :key="ctrl.title"
        :icon="ctrl.icon"
        :label="ctrl.title"
        @click="slide(ctrl.direction)"
      />
    </nav>

    <div ref="scroller" class="home-carousel overflow-x-auto overflow-y-hidden w-full pb-1">
      <div class="home-carousel-track flex gap-4">
        <slot />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { inject, onBeforeUnmount, onMounted, onUpdated, ref, watch } from 'vue'
import { BlockActionsHostKey } from '@/config/symbols'

import M3IconButton from '@/components/m3/M3IconButton.vue'

const actionsHost = inject(BlockActionsHostKey, ref(null))

const scroller = ref<HTMLDivElement>()
const hasOverflow = ref(false)

const controls = [
  { title: 'Scroll left', icon: 'chevron_left', direction: -1 as const },
  { title: 'Scroll right', icon: 'chevron_right', direction: 1 as const },
]

let resizeObserver: ResizeObserver | undefined

const updateOverflow = () => {
  const el = scroller.value

  if (!el) {
    hasOverflow.value = false
    return
  }

  hasOverflow.value = el.scrollWidth > el.clientWidth + 1
}

const observeOverflow = (el: HTMLDivElement | undefined) => {
  resizeObserver?.disconnect()
  resizeObserver = undefined

  if (!el) {
    return
  }

  resizeObserver = new ResizeObserver(updateOverflow)
  resizeObserver.observe(el)
  resizeObserver.observe(el.firstElementChild ?? el)
  updateOverflow()
}

onMounted(() => {
  observeOverflow(scroller.value)
  window.addEventListener('resize', updateOverflow)
})

onUpdated(updateOverflow)

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  window.removeEventListener('resize', updateOverflow)
})

watch(scroller, observeOverflow)

const slide = (direction: 1 | -1) => {
  const el = scroller.value
  if (!el) {
    return
  }
  const max = el.scrollWidth - el.clientWidth
  const target = Math.max(0, Math.min(max, el.scrollLeft + direction * el.clientWidth))
  el.scrollTo({ left: target, behavior: 'smooth' })
}
</script>

<style lang="postcss">
.home-carousel {
  scrollbar-width: none;
}

.home-carousel::-webkit-scrollbar {
  display: none;
}

.home-carousel-track > * {
  flex: none;
  width: 184px;
}
</style>
