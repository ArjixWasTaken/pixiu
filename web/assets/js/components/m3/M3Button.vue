<template>
  <component
    :is="href ? 'a' : 'button'"
    :href
    :type="href ? undefined : type"
    :disabled="href ? undefined : disabled"
    :aria-disabled="disabled || undefined"
    :class="[variant, size, { 'with-icon': icon || $slots.icon }]"
    class="m3-button m3-state m3-label-large"
  >
    <slot name="icon">
      <M3Icon v-if="icon" :name="icon" :size="18" />
    </slot>
    <span class="label"><slot /></span>
  </component>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

withDefaults(
  defineProps<{
    variant?: 'filled' | 'tonal' | 'outlined' | 'text' | 'elevated'
    size?: 's' | 'm'
    icon?: string
    href?: string
    type?: 'button' | 'submit' | 'reset'
    disabled?: boolean
  }>(),
  {
    variant: 'filled',
    size: 's',
    type: 'button',
    disabled: false,
  },
)
</script>

<style scoped>
.m3-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: var(--m3-button-height);
  padding: 0 var(--m3-button-pad);
  border-radius: 9999px;
  border: 0;
  cursor: pointer;
  white-space: nowrap;
  text-decoration: none;
  flex-shrink: 0;
  transition:
    box-shadow 150ms linear,
    background-color 150ms linear;

  &.with-icon {
    padding: 0 var(--m3-button-pad) 0 calc(var(--m3-button-pad) - 8px);
  }

  &.m {
    height: var(--m3-button-height-m);
    padding: 0 24px;
  }

  &.filled {
    background: var(--schemes-primary);
    color: var(--schemes-on-primary);

    @media (hover: hover) {
      &:hover {
        box-shadow: var(--m3-elevation-1);
      }
    }
  }

  &.tonal {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);

    @media (hover: hover) {
      &:hover {
        box-shadow: var(--m3-elevation-1);
      }
    }
  }

  &.elevated {
    background: var(--schemes-surface-container-low);
    color: var(--schemes-primary);
    box-shadow: var(--m3-elevation-1);

    @media (hover: hover) {
      &:hover {
        box-shadow: var(--m3-elevation-2);
      }
    }
  }

  &.outlined {
    background: transparent;
    color: var(--schemes-on-surface-variant);
    border: 1px solid var(--schemes-outline-variant);
  }

  &.text {
    background: transparent;
    color: var(--schemes-primary);
    padding: 0 12px;

    &.with-icon {
      padding: 0 16px 0 12px;
    }
  }

  &:disabled,
  &[aria-disabled='true'] {
    cursor: default;
    box-shadow: none;
    color: color-mix(in srgb, var(--schemes-on-surface) 38%, transparent);
    pointer-events: none;

    &.filled,
    &.tonal,
    &.elevated {
      background: color-mix(in srgb, var(--schemes-on-surface) 10%, transparent);
    }

    &.outlined {
      border-color: color-mix(in srgb, var(--schemes-on-surface) 12%, transparent);
    }
  }
}
</style>
