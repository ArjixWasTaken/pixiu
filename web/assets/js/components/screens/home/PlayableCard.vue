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
      <span class="title m3-body-large truncate">{{ playable.title }}</span>
      <span class="supporting m3-body-medium truncate">{{ artist }}</span>
    </span>
    <span class="trailing">
      <span class="time m3-label-medium">{{ fmtLength }}</span>
      <OfflineButton :class="{ reveal: offlineIdle }" :playable />
      <M3IconButton :icon-size="20" class="reveal" icon="more_vert" label="More actions" @click.stop="onContextMenu" />
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
import PlayableThumbnail from '@/components/playable/PlayableThumbnail.vue'
import OfflineButton from '@/components/ui/OfflineButton.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const props = defineProps<{ playable: Playable }>()
const { playable } = toRefs(props)

const { startDragging } = useDraggable('playables')
const { openContextMenu } = useContextMenu()
const { isCached, isCaching, hasCachingError } = useOfflinePlayback()

const artist = computed(() => playable.value.artist_name || '')
const playing = computed(() => ['Playing', 'Paused'].includes(playable.value.playback_state!))
/** Neither kept offline nor on its way: the button shows only with the card's other actions. */
const offlineIdle = computed(
  () => !isCached(playable.value) && !isCaching(playable.value) && !hasCachingError(playable.value),
)
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

const onDragStart = (event: DragEvent) => startDragging(event, [playable.value])

const onContextMenu = (event: MouseEvent) => {
  openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [playable.value] })
}
</script>

<style scoped>
.playable-card {
  display: flex;
  align-items: center;
  gap: var(--m3-gutter);
  min-height: var(--m3-row-height);
  padding: 4px 8px 4px 12px;
  border-radius: 12px;
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

/* With a mouse, the card's actions show when it is pointed at or focused. */
@media (hover: hover) {
  .reveal {
    opacity: 0;
    transition: opacity 100ms linear;

    .playable-card:is(:hover, :focus-within) & {
      opacity: 1;
    }
  }
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
