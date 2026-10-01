<template>
  <div
    class="mini-player"
    data-testid="mini-player"
    @click="expand"
    @pointerdown="onPointerDown"
    @pointerup="onPointerUp"
  >
    <div class="row">
      <span :style="{ backgroundImage: `url(${cover}), url(${defaultCover})` }" aria-hidden="true" class="art" />
      <div class="flex-1 min-w-0 pl-1">
        <p class="m3-title-small truncate text-(--schemes-on-surface)">{{ playable?.title ?? 'Nothing playing' }}</p>
        <p class="m3-body-small truncate text-(--schemes-on-surface-variant)">{{ artist }}</p>
      </div>
      <div class="flex items-center gap-1" @click.stop>
        <FavoriteButton v-if="playable" :favorite="playable.favorite" size="md" @toggle="toggleFavorite" />
        <PlayButton shape="round" size="s" />
        <M3IconButton fill icon="skip_next" label="Play next in queue" @click.prevent="playNext" />
      </div>
    </div>
    <div class="progress">
      <div :style="{ width: `${percent}%` }" />
    </div>
  </div>
</template>

<script lang="ts" setup>
import type { Ref } from 'vue'
import { computed, ref } from 'vue'
import { requireInjection, use } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { playableStore } from '@/stores/playableStore'
import { playback } from '@/services/playbackManager'
import { useBranding } from '@/composables/useBranding'
import { useNowPlaying } from '@/composables/useNowPlaying'
import { usePlaybackProgress } from '@/composables/usePlaybackProgress'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlayButton from '@/components/ui/FooterPlayButton.vue'

const playable = requireInjection<Ref<Playable | undefined>>(CurrentStreamableKey, ref())
const { cover: defaultCover } = useBranding()
const nowPlaying = useNowPlaying()
const { percent } = usePlaybackProgress()

const cover = computed(() => (playable.value ? playable.value.album_cover : defaultCover))
const artist = computed(() => (playable.value ? playable.value.artist_name : ''))

const toggleFavorite = () => use(playable.value, p => playableStore.toggleFavorite(p))
const playNext = () => playback().playNext()

// A tap or an upward swipe opens the full player.
let startY: number | null = null
let swiped = false

const onPointerDown = (event: PointerEvent) => {
  startY = event.clientY
  swiped = false
}

const onPointerUp = (event: PointerEvent) => {
  if (startY !== null && startY - event.clientY > 30) {
    swiped = true
    nowPlaying.show()
  }

  startY = null
}

const expand = () => swiped || (playable.value && nowPlaying.show())
</script>

<style scoped>
.mini-player {
  margin: 0 8px;
  overflow: hidden;
  border-radius: 12px;
  background: var(--schemes-surface-container-highest);
  cursor: pointer;
  touch-action: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 8px 6px;
}

.art {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  background-size: cover;
  background-position: center;
}

.progress {
  height: 3px;
  margin: 0 12px 8px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--schemes-surface-container-highest);

  div {
    height: 100%;
    border-radius: 2px;
    background: var(--schemes-primary);
  }
}
</style>
