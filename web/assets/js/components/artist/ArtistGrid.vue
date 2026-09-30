<template>
  <VirtualGridScroller
    ref="scroller"
    :items="artists"
    :min-item-width="isMobile ? 140 : 180"
    class="virtual-card-grid"
    data-testid="artist-grid"
    @scrolled-to-end="emit('scrolled-to-end')"
  >
    <template #default="{ item }: { item: Artist }">
      <ArtistCard :artist="item" />
    </template>
  </VirtualGridScroller>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { useViewport } from '@/composables/useViewport'

import VirtualGridScroller from '@/components/ui/VirtualGridScroller.vue'
import ArtistCard from '@/components/artist/ArtistCard.vue'

defineProps<{ artists: Artist[] }>()

const emit = defineEmits<{ (e: 'scrolled-to-end'): void }>()

const { isMobile } = useViewport()

const scroller = ref<InstanceType<typeof VirtualGridScroller>>()

const scrollToTop = () => scroller.value?.scrollToTop()

defineExpose({ scrollToTop })
</script>

<style>
/* Unscoped: the virtual scroller hands this class to its inner grid. */
.virtual-card-grid {
  gap: 16px;
  padding: 12px 24px 24px;

  @media (max-width: 768px) {
    gap: 12px;
    padding: 12px 16px 16px;
  }
}
</style>
