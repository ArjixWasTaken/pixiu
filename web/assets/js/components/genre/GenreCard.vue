<template>
  <li data-vue="GenreCard" draggable="true" tabindex="0" @contextmenu.prevent="onContextMenu" @dragstart="onDragStart">
    <a
      :class="`tone-${tone % 4}`"
      :href="url('genres.show', { id: genre.id })"
      :title="genre.name || 'No Genre'"
      class="genre-card m3-state"
    >
      <span :class="genre.name || 'italic'" class="m3-title-large truncate name">{{ genre.name || 'No Genre' }}</span>
      <span class="m3-body-medium count">{{ pluralize(genre.song_count, 'song') }}</span>
    </a>
  </li>
</template>

<script setup lang="ts">
import { pluralize } from '@/utils/formatters'
import { useRouter } from '@/composables/useRouter'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useContextMenu } from '@/composables/useContextMenu'
import { defineAsyncComponent } from '@/utils/helpers'

const props = withDefaults(defineProps<{ genre: Genre; tone?: number }>(), { tone: 0 })

const ContextMenu = defineAsyncComponent(() => import('@/components/genre/GenreContextMenu.vue'))

const { url } = useRouter()
const { startDragging } = useDraggable('genre')
const { openContextMenu } = useContextMenu()

const onContextMenu = (event: MouseEvent) =>
  openContextMenu<'GENRE'>(ContextMenu, event, {
    genre: props.genre,
  })

const onDragStart = (event: DragEvent) => startDragging(event, props.genre)
</script>

<style scoped>
.genre-card {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 4px;
  min-height: 128px;
  padding: 16px;
  border-radius: 12px;
  color: var(--schemes-on-surface);

  &.tone-0 {
    background: var(--schemes-primary-container);
  }

  &.tone-1 {
    background: var(--schemes-secondary-container);
  }

  &.tone-2 {
    background: var(--schemes-tertiary-container);
  }

  &.tone-3 {
    background: var(--schemes-surface-container-highest);
  }
}

.count {
  color: var(--schemes-on-surface-variant);
}
</style>
