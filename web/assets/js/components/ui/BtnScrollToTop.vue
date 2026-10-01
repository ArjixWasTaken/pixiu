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
import { onMounted, ref } from 'vue'
import { $ } from '@/utils/$'
import M3Icon from '@/components/m3/M3Icon.vue'

const el = ref<HTMLElement>()
const showing = ref(false)

const scrollToTop = () => {
  if (!el.value?.parentElement) {
    return
  }

  $.scrollTo(el.value.parentElement, 0, 500, () => (showing.value = false))
}

onMounted(() => {
  el.value?.parentElement?.addEventListener('scroll', event => {
    showing.value = (event.target as HTMLElement).scrollTop > 64
  })
})
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
