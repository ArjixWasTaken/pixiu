<template>
  <div
    :class="{ behind: nowPlaying.open.value }"
    :draggable
    class="song-info"
    data-vue="FooterPlayableInfo"
    @dragstart="onDragStart"
  >
    <button
      :aria-label="nowPlaying.open.value ? 'Collapse player' : 'Expand player'"
      :style="{ backgroundImage: `url(${cover}), url(${defaultCover})` }"
      class="album-thumb"
      type="button"
      @click="nowPlaying.toggle"
    />
    <div v-if="playable" class="meta">
      <p class="title m3-title-medium" @click="nowPlaying.toggle">{{ playable.title }}</p>
      <a :href="artistUri" class="artist m3-body-medium">{{ artistName }}</a>
    </div>
    <FavoriteButton v-if="playable" :favorite="playable.favorite" size="md" @toggle="toggleFavorite" />
  </div>
</template>

<script lang="ts" setup>
import { useViewport } from '@/composables/useViewport'
import type { Ref } from 'vue'
import { computed, ref } from 'vue'
import { requireInjection, use } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { usePlayableStore } from '@/stores/playableStore'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { useBranding } from '@/composables/useBranding'
import { useNowPlaying } from '@/composables/useNowPlaying'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'

const playableStore = usePlayableStore()

const { isTouch } = useViewport()

const { startDragging } = useDraggable('playables')
const { url } = useRouter()
const { cover: defaultCover } = useBranding()
const nowPlaying = useNowPlaying()

const playable = requireInjection<Ref<Playable | undefined>>(CurrentStreamableKey, ref())

const cover = computed(() => (playable.value ? playable.value.album_cover : defaultCover))

const artistUri = computed(() => (playable.value ? url('artists.show', { id: playable.value.artist_id }) : ''))

const artistName = computed(() => (playable.value ? playable.value.artist_name : ''))

const draggable = computed(() => Boolean(playable.value) && !isTouch.value)

const onDragStart = (event: DragEvent) => use(playable.value, p => startDragging(event, [p]))
const toggleFavorite = () => use(playable.value, p => playableStore.toggleFavorite(p))
</script>

<style scoped>
.song-info {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 300px;
  min-width: 0;
  flex-shrink: 0;
}

/* The expanded player shows all of this, large: the bar leaves it out meanwhile. */
.behind > * {
  visibility: hidden;
}

.album-thumb {
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  border-radius: 12px;
  background-size: cover;
  background-position: center;
  cursor: pointer;

  @media (pointer: fine) and (min-width: 769px) {
    width: 48px;
    height: 48px;
    border-radius: 6px;
  }
}

.meta {
  flex: 1;
  min-width: 0;
}

.title,
.artist {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.title {
  color: var(--schemes-on-surface);
  cursor: pointer;
}

.artist {
  color: var(--schemes-on-surface-variant);
}
</style>
