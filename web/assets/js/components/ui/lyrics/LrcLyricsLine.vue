<template>
  <p :class="{ active: isActive }" class="lrc-line relative origin-left transition-all duration-200 cursor-default">
    {{ line.text }}
  </p>
</template>

<script lang="ts" setup>
defineProps<{
  line: LrcLine
  isActive: boolean
}>()
</script>

<style scoped>
.lrc-line {
  color: var(--schemes-on-surface);
  line-height: 1.33;

  &.active {
    color: var(--schemes-primary);
  }
}
</style>
