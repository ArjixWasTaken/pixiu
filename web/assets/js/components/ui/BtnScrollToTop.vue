<template>
  <Transition name="fade">
    <button
      v-show="showing"
      ref="el"
      class="sm:hidden block fixed right-[1.8rem] z-20 opacity-100 duration-500 transition-opacity rounded-full py-2 px-4 bg-(--schemes-surface-container-highest) text-(--schemes-on-surface)"
      title="Scroll to top"
      type="button"
      @click="scrollToTop"
    >
      <M3Icon name="arrow_circle_up" />&nbsp; Top
    </button>
  </Transition>
</template>

<script lang="ts" setup>
import { useScroll } from '@vueuse/core'
import { computed, onMounted, shallowRef, useTemplateRef } from 'vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const el = useTemplateRef('el')

/** The screen it sits on scrolls; it watches that. */
const scroller = shallowRef<HTMLElement | null>(null)
onMounted(() => (scroller.value = el.value?.closest<HTMLElement>('.screen-body') ?? null))

const { y } = useScroll(scroller)
const showing = computed(() => y.value > 64)

const scrollToTop = () => scroller.value?.scrollTo({ top: 0, behavior: 'smooth' })
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
button {
  @apply border border-(--schemes-outline) text-(--schemes-on-surface);
  bottom: calc(var(--footer-height) + 26px);

  &.fade-enter,
  &.fade-leave-to {
    @apply opacity-0;
  }
}
</style>
