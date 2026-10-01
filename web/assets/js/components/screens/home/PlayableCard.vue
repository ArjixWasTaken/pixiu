<template>
  <li
    :class="{ playing }"
    class="playable-card m3-state"
    data-testid="song-card"
    data-vue="PlayableCard"
    draggable="true"
    tabindex="0"
    @click="play"
    @contextmenu.prevent="onContextMenu"
    @dragstart="onDragStart"
    @keydown.enter.self.prevent="play"
  >
    <PlayableThumbnail :playable @clicked="play" />
    <span class="flex-1 min-w-0 flex flex-col">
      <span class="title m3-body-large flex gap-2 items-center">
        <M3Icon
          v-if="cachingOffline"
          :size="16"
          class="opacity-60"
          name="progress_activity"
          title="Caching for offline playback"
        />
        <M3Icon
          v-else-if="cachingFailed"
          :size="16"
          :title="`Error: ${cachingErrorMessage}`"
          class="text-(--schemes-error)"
          name="error"
        />
        <OfflineMark v-else-if="cachedOffline" />
        <span class="truncate">{{ playable.title }}</span>
      </span>
      <span class="supporting m3-body-medium truncate">{{ artist }}</span>
    </span>
    <span class="trailing">
      <span class="time m3-label-medium">{{ fmtLength }}</span>
      <FavoriteButton :favorite="playable.favorite" @toggle="toggleFavorite" />
      <M3IconButton :icon-size="20" icon="more_vert" label="More actions" @click.stop="onContextMenu" />
    </span>
  </li>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { secondsToHis } from '@/utils/formatters'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useContextMenu } from '@/composables/useContextMenu'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { playback } from '@/services/playbackManager'
import { playableStore } from '@/stores/playableStore'

import PlayableThumbnail from '@/components/playable/PlayableThumbnail.vue'
import OfflineMark from '@/components/ui/OfflineMark.vue'
import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const props = defineProps<{ playable: Playable }>()
const { playable } = toRefs(props)

const { startDragging } = useDraggable('playables')
const { openContextMenu } = useContextMenu()
const { isCached, isCaching, hasCachingError, getCachingError } = useOfflinePlayback()

const artist = computed(() => playable.value.artist_name || '')
const playing = computed(() => ['Playing', 'Paused'].includes(playable.value.playback_state!))
const cachedOffline = computed(() => isCached(playable.value))
const cachingOffline = computed(() => isCaching(playable.value))
const cachingFailed = computed(() => hasCachingError(playable.value))
const cachingErrorMessage = computed(() => getCachingError(playable.value))
const fmtLength = secondsToHis(playable.value.length)

const play = () => {
  if (playable.value.playback_state === 'Playing') {
    playback().pause()
  } else if (playable.value.playback_state === 'Paused') {
    playback().resume()
  } else {
    playback().play(playable.value)
  }
}

const toggleFavorite = () => playableStore.toggleFavorite(playable.value)

const onDragStart = (event: DragEvent) => startDragging(event, [playable.value])

const onContextMenu = (event: MouseEvent) => {
  openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [playable.value] })
}
</script>

<style scoped>
.playable-card {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 72px;
  padding: 8px 8px 8px 16px;
  border-radius: 16px;
  color: var(--schemes-on-surface);
  cursor: pointer;
  list-style: none;

  &.playing .title {
    color: var(--schemes-primary);
  }
}

.supporting {
  color: var(--schemes-on-surface-variant);
}

.trailing {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  color: var(--schemes-on-surface-variant);
}

.time {
  min-width: 44px;
  margin-right: 4px;
  text-align: right;
  font-variant-numeric: tabular-nums;

  @media (max-width: 768px) {
    display: none;
  }
}
</style>
