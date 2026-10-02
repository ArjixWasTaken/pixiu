<template>
  <template v-if="isMobile">
    <li class="sheet-header" role="none" tabindex="-1">
      <span
        :class="{ round }"
        :style="{ backgroundImage: `url(${coverOfSize(cover, 128) || defaultCover}), url(${defaultCover})` }"
        class="sheet-cover"
      />
      <span class="flex-1 min-w-0">
        <span class="m3-title-medium block truncate">{{ title }}</span>
        <span v-if="subtitle" class="m3-body-medium block truncate text-(--schemes-on-surface-variant)">
          {{ subtitle }}
        </span>
      </span>
      <slot />
      <M3IconButton icon="close" label="Close" @click.stop="closeContextMenu" />
    </li>
    <li class="separator" role="separator" />
  </template>
</template>

<script lang="ts" setup>
import { useBranding } from '@/composables/useBranding'
import { useContextMenu } from '@/composables/useContextMenu'
import { useViewport } from '@/composables/useViewport'
import { coverOfSize } from '@/services/subsonic'

import M3IconButton from '@/components/m3/M3IconButton.vue'

/** On a phone, where a menu is a sheet: what it's about (cover, name), and a way to close it. */
withDefaults(defineProps<{ cover?: string | null; title: string; subtitle?: string; round?: boolean }>(), {
  cover: null,
  subtitle: '',
  round: false,
})

const { isMobile } = useViewport()
const { cover: defaultCover } = useBranding()
const { closeContextMenu } = useContextMenu()
</script>

<style scoped>
.sheet-header {
  gap: 12px !important;
  padding: 0 8px 12px 20px !important;
  cursor: default !important;

  @media (hover: hover) {
    &:hover {
      background: transparent !important;
    }
  }
}

.sheet-cover {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  background-size: cover;
  background-position: center;

  &.round {
    border-radius: 50%;
  }
}
</style>
