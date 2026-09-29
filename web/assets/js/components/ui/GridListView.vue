<template>
  <div ref="container" :class="`as-${viewMode}`" class="card-grid">
    <slot />
  </div>
</template>

<script lang="ts" setup>
import { nextTick, ref, toRefs } from 'vue'

const props = withDefaults(defineProps<{ viewMode?: ViewMode }>(), {
  viewMode: 'grid',
})

const container = ref<HTMLDivElement>()

const { viewMode } = toRefs(props)

const scrollToTop = async () => {
  await nextTick()

  container.value!.scrollTo?.({
    top: 0,
    behavior: 'smooth',
  })
}

defineExpose({
  scrollToTop,
})
</script>

<style lang="postcss" scoped>
.card-grid {
  display: grid;
  gap: 16px;
  padding: 12px 24px 24px;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  content-visibility: auto;

  @media (max-width: 768px) {
    gap: 12px;
    padding: 12px 16px 16px;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  }

  &.as-list {
    align-content: start;
    align-items: start;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  }
}
</style>
