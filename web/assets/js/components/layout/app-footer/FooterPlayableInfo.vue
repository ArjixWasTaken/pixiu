<template>
  <div :draggable class="song-info" data-vue="FooterPlayableInfo" @dragstart="onDragStart">
    <button
      :aria-label="nowPlaying.open.value ? 'Collapse player' : 'Expand player'"
      :style="{ backgroundImage: `url(${cover}), url(${defaultCover})` }"
      class="album-thumb"
      type="button"
      @click="nowPlaying.toggle"
    />
    <div v-if="playable" class="meta">
      <p class="title m3-title-medium" @click="nowPlaying.toggle">{{ playable.title }}</p>
      <a :href="artistOrPodcastUri" class="artist m3-body-medium">{{ artistOrPodcastName }}</a>
    </div>
    <FavoriteButton v-if="playable" :favorite="playable.favorite" size="md" @toggle="toggleFavorite" />
  </div>
</template>

<script lang="ts" setup>
import isMobile from 'ismobilejs'
import type { Ref } from 'vue'
import { computed, ref } from 'vue'
import { getPlayableProp, requireInjection, use } from '@/utils/helpers'
import { isSong } from '@/utils/typeGuards'
import { CurrentStreamableKey } from '@/config/symbols'
import { playableStore } from '@/stores/playableStore'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { useBranding } from '@/composables/useBranding'
import { useNowPlaying } from '@/composables/useNowPlaying'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'

const { startDragging } = useDraggable('playables')
const { url } = useRouter()
const { cover: defaultCover } = useBranding()
const nowPlaying = useNowPlaying()

const playable = requireInjection<Ref<Playable | undefined>>(CurrentStreamableKey, ref())

const cover = computed(() =>
  playable.value ? getPlayableProp(playable.value, 'album_cover', 'episode_image') : defaultCover,
)

const artistOrPodcastUri = computed(() => {
  if (!playable.value) {
    return ''
  }

  return isSong(playable.value)
    ? url('artists.show', { id: playable.value?.artist_id })
    : url('podcasts.show', { id: playable.value?.podcast_id })
})

const artistOrPodcastName = computed(() =>
  playable.value ? getPlayableProp(playable.value, 'artist_name', 'podcast_title') : '',
)

const draggable = computed(() => Boolean(playable.value) && !isMobile.any)

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

.album-thumb {
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  border-radius: 12px;
  background-size: cover;
  background-position: center;
  cursor: pointer;
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
