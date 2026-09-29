<template>
  <span
    v-if="variant === 'linear'"
    :aria-valuenow="value === undefined ? undefined : Math.round(value * 100)"
    :class="{ indeterminate: value === undefined }"
    class="m3-linear"
    aria-valuemax="100"
    aria-valuemin="0"
    role="progressbar"
  >
    <span :style="{ width: value === undefined ? undefined : `${value * 100}%` }" class="bar" />
  </span>
  <svg
    v-else
    :aria-valuenow="value === undefined ? undefined : Math.round(value * 100)"
    :class="{ indeterminate: value === undefined }"
    :height="size"
    :width="size"
    class="m3-circular"
    role="progressbar"
    viewBox="0 0 48 48"
  >
    <circle class="track" cx="24" cy="24" r="20" />
    <circle :stroke-dasharray="dash" class="indicator" cx="24" cy="24" r="20" />
  </svg>
</template>

<script lang="ts" setup>
import { computed } from 'vue'

const props = withDefaults(defineProps<{ variant?: 'linear' | 'circular'; value?: number; size?: number }>(), {
  variant: 'linear',
  value: undefined,
  size: 48,
})

const circumference = 2 * Math.PI * 20
const dash = computed(() => `${(props.value ?? 0.25) * circumference} ${circumference}`)
</script>

<style scoped>
.m3-linear {
  position: relative;
  display: block;
  height: 4px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--schemes-secondary-container);

  .bar {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    border-radius: 2px;
    background: var(--schemes-primary);
    transition: width 200ms linear;
  }

  &.indeterminate .bar {
    animation: m3-indeterminate 1.4s var(--m3-ease) infinite;
  }
}

.m3-circular {
  transform: rotate(-90deg);
  flex-shrink: 0;

  circle {
    fill: none;
    stroke-width: 4;
    stroke-linecap: round;
  }

  .track {
    stroke: var(--schemes-secondary-container);
  }

  .indicator {
    stroke: var(--schemes-primary);
    transition: stroke-dasharray 200ms linear;
  }

  &.indeterminate {
    animation: m3-spin 1s linear infinite;

    .track {
      stroke: transparent;
    }
  }
}
</style>
