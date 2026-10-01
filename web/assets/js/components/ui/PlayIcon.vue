<template>
  <span
    :class="size"
    class="play-icon absolute flex opacity-0 no-hover:opacity-100 items-center justify-center aspect-square rounded-full top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-(--schemes-primary) group-hover:opacity-100 duration-500 transition z-20"
    aria-hidden="true"
  >
    <M3Icon
      v-if="playing"
      name="pause"
      fill
      data-testid="icon-pause"
      class="text-(--schemes-on-primary)"
      :size="iconSize"
    />
    <M3Icon
      v-else
      name="play_arrow"
      fill
      data-testid="icon-play"
      class="ml-0.5 text-(--schemes-on-primary)"
      :size="iconSize"
    />
  </span>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(defineProps<{ playing?: boolean; size?: 'sm' | 'lg' }>(), {
  playing: false,
  size: 'lg',
})

const iconSize = computed(() => (props.size === 'lg' ? 24 : 20))
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.play-icon.sm {
  @apply w-[24px];
}

.play-icon.lg {
  @apply w-[32px];
}
</style>
