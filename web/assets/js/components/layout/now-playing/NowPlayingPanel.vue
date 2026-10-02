<template>
  <Transition name="panel">
    <section
      v-if="nowPlaying.open.value && song"
      class="now-playing-panel"
      data-testid="now-playing"
      data-vue="NowPlaying"
    >
      <div class="stage">
        <div :style="{ backgroundImage: `url(${cover}), url(${defaultCover})` }" class="art">
          <PlatformBadge :platform="song?.source_platform" size="xl" />
        </div>
        <div class="caption">
          <div class="flex-1 min-w-0">
            <p class="m3-headline-small truncate text-(--schemes-on-surface)">{{ song.title }}</p>
            <a :href="url('artists.show', { id: song.artist_id })" class="m3-body-large block truncate sub">{{
              subtitle
            }}</a>
          </div>
          <M3IconButton icon="playlist_add" label="Save to playlist" size="m" @click="openMenu" />
          <FavoriteButton :favorite="song.favorite" size="md" @toggle="toggleFavorite" />
        </div>
      </div>

      <div class="pane">
        <div class="tabs">
          <M3Tabs v-model="nowPlaying.tab.value" :tabs secondary />
        </div>
        <div class="pane-body" data-scrolls-lists>
          <NowPlayingQueue v-if="nowPlaying.tab.value === 'queue'" />
          <NowPlayingLyrics v-else-if="nowPlaying.tab.value === 'lyrics'" :song />
          <NowPlayingAbout v-else :song />
        </div>
      </div>
    </section>
  </Transition>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { requireInjection, defineAsyncComponent } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { usePlayableStore } from '@/stores/playableStore'
import { useBranding } from '@/composables/useBranding'
import { useContextMenu } from '@/composables/useContextMenu'
import { useNowPlaying } from '@/composables/useNowPlaying'
import { useRouter } from '@/composables/useRouter'
import type { M3Tab } from '@/components/m3/M3Tabs.vue'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Tabs from '@/components/m3/M3Tabs.vue'
import NowPlayingAbout from '@/components/layout/now-playing/NowPlayingAbout.vue'
import NowPlayingLyrics from '@/components/layout/now-playing/NowPlayingLyrics.vue'
import NowPlayingQueue from '@/components/layout/now-playing/NowPlayingQueue.vue'
import PlatformBadge from '@/components/ui/PlatformBadge.vue'

const playableStore = usePlayableStore()

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const nowPlaying = useNowPlaying()
const { url, onRouteChanged } = useRouter()
const { cover: defaultCover } = useBranding()
const { openContextMenu } = useContextMenu()

const streamable = requireInjection(CurrentStreamableKey, ref())
const song = computed(() => (streamable.value ? streamable.value : null))

const tabs: M3Tab[] = [
  { id: 'queue', label: 'Up next' },
  { id: 'lyrics', label: 'Lyrics' },
  { id: 'about', label: 'About' },
]

const cover = computed(() => song.value?.album_cover || defaultCover)

const subtitle = computed(() =>
  song.value
    ? [song.value.artist_name, song.value.is_single ? null : song.value.album_name, song.value.year]
        .filter(Boolean)
        .join(' · ')
    : '',
)

const toggleFavorite = () => song.value && playableStore.toggleFavorite(song.value)

const openMenu = (event: MouseEvent) =>
  song.value && openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [song.value] })

// Going somewhere else puts the player away.
onRouteChanged(() => nowPlaying.close())
</script>

<style scoped>
.now-playing-panel {
  position: absolute;
  inset: 0;
  z-index: 30;
  display: flex;
  gap: 48px;
  padding: 24px 40px;
  overflow: hidden;
  background: var(--schemes-surface);
}

.stage {
  flex: 1 1 0%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 24px;
  /* The art is sized from the room here, both ways, so it stays square. */
  container-type: size;
}

.art {
  position: relative;
  width: min(100%, 640px, 100cqh - 96px);
  aspect-ratio: 1 / 1;
  border-radius: 28px;
  background-size: cover;
  background-position: center;
  box-shadow: var(--m3-elevation-3);
}

.caption {
  width: min(100%, 640px, 100cqh - 96px);
  display: flex;
  align-items: center;
  gap: 8px;
}

.sub {
  color: var(--schemes-on-surface-variant);
}

.pane {
  width: min(44%, 520px);
  flex-shrink: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.tabs {
  flex-shrink: 0;
  border-bottom: 1px solid var(--schemes-outline-variant);
}

.pane-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 12px 0 8px;
}

.panel-enter-active,
.panel-leave-active {
  transition:
    opacity 200ms linear,
    transform 250ms var(--m3-ease);
}

.panel-enter-from,
.panel-leave-to {
  opacity: 0;
  transform: translateY(24px);
}
</style>
