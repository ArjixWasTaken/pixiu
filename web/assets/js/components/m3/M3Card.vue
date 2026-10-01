<template>
  <component :is="tag" :class="[variant, { 'm3-state interactive': interactive }]" class="m3-card">
    <slot />
  </component>
</template>

<script lang="ts" setup>
withDefaults(
  defineProps<{
    /** `plain`: no container at all, only the hover tint (as for covers with a caption). */
    variant?: 'filled' | 'elevated' | 'outlined' | 'plain'
    interactive?: boolean
    tag?: string
  }>(),
  {
    variant: 'filled',
    interactive: false,
    tag: 'div',
  },
)
</script>

<style scoped>
.m3-card {
  border-radius: 12px;
  color: var(--schemes-on-surface);
  text-align: start;
  transition: box-shadow 150ms linear;

  &.filled {
    background: var(--schemes-surface-container-highest);
  }

  &.elevated {
    background: var(--schemes-surface-container-low);
    box-shadow: var(--m3-elevation-1);
  }

  &.outlined {
    background: var(--schemes-surface);
    border: 1px solid var(--schemes-outline-variant);
  }

  &.plain {
    background: transparent;
  }

  &.interactive {
    cursor: pointer;

    &:not(.plain):hover {
      box-shadow: var(--m3-elevation-1);
    }

    &.elevated:hover {
      box-shadow: var(--m3-elevation-2);
    }
  }
}
</style>
