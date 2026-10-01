<template>
  <M3Card
    :class="layout"
    :draggable="!isTouch"
    class="album-artist-card"
    data-testid="artist-album-card"
    interactive
    variant="plain"
    tabindex="0"
    @click="open"
    @contextmenu.prevent="onContextMenu"
    @dblclick="onDblClick"
    @dragstart="onDragStart"
    @keydown.enter.self="open"
  >
    <slot name="thumbnail">
      <Thumbnail :entity />
    </slot>

    <footer class="text">
      <slot name="name" />
      <p v-if="$slots.meta" class="m3-body-medium meta">
        <slot name="meta" />
      </p>
    </footer>

    <slot />
  </M3Card>
</template>

<script lang="ts" setup>
import { useViewport } from '@/composables/useViewport'
import { toRefs } from 'vue'
import { useRouter } from '@/composables/useRouter'

import Thumbnail from '@/components/ui/album-artist/AlbumOrArtistThumbnail.vue'
import M3Card from '@/components/m3/M3Card.vue'

const { isTouch } = useViewport()

const props = withDefaults(defineProps<{ layout?: CardLayout; entity: Artist | Album; href?: string }>(), {
  layout: 'full',
})

const emit = defineEmits<{
  (e: 'dblclick'): void
  (e: 'dragstart', event: DragEvent): void
  (e: 'contextmenu', event: MouseEvent): void
}>()

const { layout } = toRefs(props)
const { go } = useRouter()

/** The whole card opens the album or artist; links and buttons inside do their own thing. */
const open = (event: Event) => {
  if (!props.href || (event.target as HTMLElement).closest('a, button')) {
    return
  }

  go(props.href)
}

const onDblClick = () => emit('dblclick')
const onDragStart = (e: DragEvent) => emit('dragstart', e)
const onContextMenu = (e: MouseEvent) => emit('contextmenu', e)
</script>

<style lang="postcss" scoped>
.album-artist-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  padding: 8px;
  outline: none;

  &:focus-visible {
    outline: 2px solid var(--schemes-secondary);
  }

  @media (max-width: 768px) {
    padding: 0;
  }

  &.compact {
    flex-direction: row;
    align-items: center;
    gap: 16px;

    :deep(.card-thumbnail) {
      width: 80px;
      flex-shrink: 0;
    }
  }
}

.text {
  min-width: 0;
  padding: 0 4px 4px;
  margin-top: -4px;

  :deep(p),
  :deep(a) {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :deep(.title) {
    color: var(--schemes-on-surface);
  }

  :deep(.subtitle) {
    color: var(--schemes-on-surface-variant);
  }
}

.meta {
  color: var(--schemes-on-surface-variant);
}
</style>
