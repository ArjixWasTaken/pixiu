<template>
  <span :data-icon="name" :style aria-hidden="true" class="material-symbols-outlined m3-icon" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'

/**
 * A Material Symbols glyph, like the design's `sym()`. The ligature comes from
 * CSS, so the icon's name stays out of the text of labels and copied text.
 */
const props = withDefaults(defineProps<{ name: string; size?: number; fill?: boolean }>(), {
  size: 24,
  fill: false,
})

const style = computed(() => ({
  fontSize: `${props.size}px`,
  width: `${props.size}px`,
  height: `${props.size}px`,
  fontVariationSettings: `'FILL' ${props.fill ? 1 : 0}, 'wght' 400, 'opsz' ${Math.min(48, Math.max(20, props.size))}`,
}))
</script>

<style scoped>
.m3-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  user-select: none;

  &::before {
    content: attr(data-icon);
  }
}
</style>
