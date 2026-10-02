<template>
  <li
    :class="{ sheet: isMobile }"
    class="rating-item"
    tabindex="-1"
    @mouseover="($event.currentTarget as HTMLLIElement).focus()"
  >
    <template v-if="isMobile">
      <M3Icon class="text-(--schemes-on-surface-variant)" name="star" />
      <span class="flex-1">Rating</span>
    </template>
    <StarRating :rateable :size="isMobile ? 'lg' : 'sm'" @rate="closeContextMenu" />
  </li>
</template>

<script lang="ts" setup>
import { useContextMenu } from '@/composables/useContextMenu'
import { useViewport } from '@/composables/useViewport'

import M3Icon from '@/components/m3/M3Icon.vue'
import StarRating from '@/components/ui/StarRating.vue'

/** A menu's rating stars: in line with the items; on a phone's sheet, labeled and sized for a finger. */
defineProps<{ rateable: Song | Album | Artist }>()

const { isMobile } = useViewport()
const { closeContextMenu } = useContextMenu()
</script>

<style scoped>
.rating-item {
  cursor: default !important;

  &:focus {
    outline: none;
  }

  @media (hover: hover) {
    &:hover {
      background: transparent !important;
    }
  }

  &.sheet {
    padding-right: 8px !important;
  }
}
</style>
