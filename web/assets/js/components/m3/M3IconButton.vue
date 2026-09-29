<template>
  <button
    :aria-label="label"
    :aria-pressed="selected === undefined ? undefined : selected"
    :title="label"
    :disabled
    :class="[variant, size, shape, { selected, toggle: selected !== undefined }]"
    class="m3-icon-button m3-state"
    type="button"
  >
    <slot>
      <M3Icon v-if="icon" :name="icon" :fill="fill ?? selected === true" :size="iconSize" />
    </slot>
  </button>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(
  defineProps<{
    icon?: string
    label?: string
    variant?: 'standard' | 'filled' | 'tonal' | 'outlined'
    size?: 'xs' | 's' | 'm' | 'l'
    shape?: 'round' | 'square'
    selected?: boolean
    fill?: boolean
    /** Overrides the icon size that goes with the button size. */
    iconSize?: number
    disabled?: boolean
  }>(),
  {
    variant: 'standard',
    size: 's',
    shape: 'round',
    selected: undefined,
    fill: undefined,
    iconSize: undefined,
    disabled: false,
  },
)

const iconSize = computed(() => props.iconSize ?? { xs: 20, s: 24, m: 24, l: 32 }[props.size])
</script>

<style scoped>
.m3-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 0;
  border-radius: 9999px;
  background: transparent;
  color: var(--schemes-on-surface-variant);
  cursor: pointer;
  transition:
    border-radius 200ms var(--m3-ease),
    background-color 150ms linear;

  &.xs {
    width: 32px;
    height: 32px;
  }

  &.s {
    width: 40px;
    height: 40px;
  }

  &.m {
    width: 56px;
    height: 56px;
  }

  &.l {
    width: 96px;
    height: 96px;
  }

  &.square {
    border-radius: 12px;

    &.m {
      border-radius: 16px;
    }

    &.l {
      border-radius: 28px;
    }
  }

  &.standard.selected {
    color: var(--schemes-primary);
  }

  &.filled {
    background: var(--schemes-primary);
    color: var(--schemes-on-primary);

    &.toggle:not(.selected) {
      background: var(--schemes-surface-container-highest);
      color: var(--schemes-primary);
    }
  }

  &.tonal {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);

    &.toggle:not(.selected) {
      background: var(--schemes-surface-container-highest);
      color: var(--schemes-on-surface-variant);
    }
  }

  &.outlined {
    border: 1px solid var(--schemes-outline-variant);

    &.selected {
      border: 0;
      background: var(--schemes-inverse-surface);
      color: var(--schemes-inverse-on-surface);
    }
  }

  &:disabled {
    cursor: default;
    color: color-mix(in srgb, var(--schemes-on-surface) 38%, transparent);

    &.filled,
    &.tonal {
      background: color-mix(in srgb, var(--schemes-on-surface) 10%, transparent);
    }
  }
}
</style>
