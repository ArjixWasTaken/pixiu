<template>
  <img
    v-if="info"
    :class="size"
    :src="info.logo"
    :title="`From ${info.name}`"
    alt=""
    aria-hidden="true"
    class="badge"
  />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { platforms } from '@/config/platforms'

/**
 * The mark of the platform a song or album was downloaded from, in the
 * top-left corner of its cover; nothing for uploads. The parent positions
 * relative to the cover. Song info says the same in words, so this is
 * hidden from screen readers.
 */
const props = withDefaults(defineProps<{ platform?: string | null; size?: 'sm' | 'md' | 'lg' | 'xl' }>(), {
  platform: null,
  size: 'sm',
})

const info = computed(() => (props.platform ? platforms[props.platform] : undefined))
</script>

<style scoped>
.badge {
  position: absolute;
  top: var(--inset);
  left: var(--inset);
  width: var(--size);
  height: var(--size);
  border-radius: 50%;
  /* Stands out on any artwork. */
  box-shadow: 0 0 0 1.5px var(--schemes-surface);
  pointer-events: none;
}

.sm {
  --size: 14px;
  --inset: 3px;
}

.md {
  --size: 20px;
  --inset: 6px;
}

.lg {
  --size: 24px;
  --inset: 8px;
}

/* The full player's cover, clear of its rounder corners. */
.xl {
  --size: 28px;
  --inset: 16px;
}
</style>
