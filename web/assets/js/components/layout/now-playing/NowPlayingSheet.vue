<template>
  <Transition name="sheet">
    <section
      v-if="nowPlaying.open.value && song"
      ref="sheet"
      class="now-playing-sheet"
      data-testid="now-playing"
      data-vue="NowPlaying"
    >
      <header
        class="handle"
        @pointercancel="onPointerUp"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
      >
        <M3IconButton icon="keyboard_arrow_down" label="Collapse player" size="m" @click="nowPlaying.close" />
        <span class="m3-label-large text-(--schemes-on-surface-variant)">{{ heading }}</span>
        <M3IconButton icon="more_vert" label="More actions" size="m" @click="openMenu" />
      </header>

      <div v-if="!pane" class="player">
        <div class="stage">
          <div :style="{ backgroundImage: `url(${cover}), url(${defaultCover})` }" class="art" />
        </div>
        <div class="flex flex-col gap-3 shrink-0">
          <div class="flex items-center gap-2">
            <div class="flex-1 min-w-0">
              <a :href="url('artists.show', { id: song.artist_id })" class="m3-headline-small block truncate title">
                {{ song.title }}
              </a>
              <p class="m3-body-large truncate sub">{{ song.artist_name }}</p>
            </div>
            <M3IconButton icon="playlist_add" label="Save to playlist" size="m" @click="openMenu" />
            <FavoriteButton :favorite="song.favorite" size="md" @toggle="toggleFavorite" />
          </div>
          <AudioPlayer class="max-w-none!" />
          <NowPlayingControls />
        </div>
      </div>

      <div v-else class="compact">
        <div class="min-w-0">
          <p class="m3-title-large truncate text-(--schemes-on-surface)">{{ song.title }}</p>
          <p class="m3-body-medium sub">{{ song.artist_name }}</p>
        </div>
        <AudioPlayer class="max-w-none!" />
        <NowPlayingControls />
      </div>

      <div :class="{ open: pane }" class="tabs">
        <M3Tabs :model-value="pane ? nowPlaying.tab.value : ''" :tabs secondary @update:model-value="choose" />
      </div>

      <div v-if="pane" class="pane-body">
        <NowPlayingQueue v-if="nowPlaying.tab.value === 'queue'" />
        <NowPlayingLyrics v-else-if="nowPlaying.tab.value === 'lyrics'" :song />
        <NowPlayingAbout v-else :song />
      </div>
    </section>
  </Transition>
</template>

<script lang="ts" setup>
import { computed, ref, useTemplateRef, watch } from 'vue'
import { requireInjection, defineAsyncComponent } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { playableStore } from '@/stores/playableStore'
import { useBranding } from '@/composables/useBranding'
import { useContextMenu } from '@/composables/useContextMenu'
import { useNowPlaying } from '@/composables/useNowPlaying'
import type { NowPlayingTab } from '@/composables/useNowPlaying'
import { useRouter } from '@/composables/useRouter'
import type { M3Tab } from '@/components/m3/M3Tabs.vue'

import AudioPlayer from '@/components/layout/app-footer/AudioPlayer.vue'
import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Tabs from '@/components/m3/M3Tabs.vue'
import NowPlayingAbout from '@/components/layout/now-playing/NowPlayingAbout.vue'
import NowPlayingLyrics from '@/components/layout/now-playing/NowPlayingLyrics.vue'
import NowPlayingControls from '@/components/layout/now-playing/NowPlayingControls.vue'
import NowPlayingQueue from '@/components/layout/now-playing/NowPlayingQueue.vue'

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const nowPlaying = useNowPlaying()
const { url, onRouteChanged } = useRouter()
const { cover: defaultCover } = useBranding()
const { openContextMenu } = useContextMenu()

const streamable = requireInjection(CurrentStreamableKey, ref())
const song = computed(() => (streamable.value ? streamable.value : null))
const cover = computed(() => song.value?.album_cover || defaultCover)

/** Whether a tab (Up next, Lyrics, About) has taken over the sheet. */
const pane = ref(false)

const tabs: M3Tab[] = [
  { id: 'queue', label: 'Up next' },
  { id: 'lyrics', label: 'Lyrics' },
  { id: 'about', label: 'About' },
]

const heading = computed(() =>
  pane.value ? ({ queue: 'Up next', lyrics: 'Lyrics', about: 'About' } as const)[nowPlaying.tab.value] : 'Now playing',
)

const choose = (tab: string | undefined) => {
  if (pane.value && nowPlaying.tab.value === tab) {
    pane.value = false
    return
  }

  nowPlaying.tab.value = (tab ?? 'queue') as NowPlayingTab
  pane.value = true
}

watch(nowPlaying.open, open => open || (pane.value = false))
onRouteChanged(() => nowPlaying.close())

const toggleFavorite = () => song.value && playableStore.toggleFavorite(song.value)

const openMenu = (event: MouseEvent) =>
  song.value && openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [song.value] })

// Dragging the handle down puts the sheet away.
const sheet = useTemplateRef('sheet')
let drag: { y: number; t: number; dy: number } | null = null

const onPointerDown = (event: PointerEvent) => {
  drag = { y: event.clientY, t: Date.now(), dy: 0 }
  ;(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId)
}

const onPointerMove = (event: PointerEvent) => {
  if (!drag || !sheet.value) {
    return
  }

  drag.dy = Math.max(0, event.clientY - drag.y)
  sheet.value.style.transition = 'none'
  sheet.value.style.transform = `translateY(${drag.dy}px)`
}

const onPointerUp = () => {
  const current = drag
  drag = null

  if (!current || !sheet.value) {
    return
  }

  sheet.value.style.transition = ''
  sheet.value.style.transform = ''

  if (current.dy > 120 || current.dy / Math.max(1, Date.now() - current.t) > 0.6) {
    nowPlaying.close()
  }
}
</script>

<style scoped>
.now-playing-sheet {
  /* The whole screen: what plays deserves it, and the navigation below
     would only compete with the controls. */
  position: fixed;
  inset: 0;
  z-index: 800;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: env(safe-area-inset-top) 0 env(safe-area-inset-bottom);
  background: var(--schemes-surface);
  box-shadow: 0 -2px 6px rgb(0 0 0 / 0.25);
  transition: transform 220ms var(--m3-ease);
}

.handle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px 0;
  flex-shrink: 0;
  touch-action: none;
}

.player {
  position: relative;
  flex: 1 1 0%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 4px 24px 0;
}

.stage {
  flex: 1 1 0%;
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 16px;
}

.art {
  height: 100%;
  max-height: 340px;
  max-width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 28px;
  background-size: cover;
  background-position: center;
  box-shadow: var(--m3-elevation-3);
}

.title {
  color: var(--schemes-on-surface);
}

.sub {
  color: var(--schemes-on-surface-variant);
}

.compact {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px 24px 0;
  flex-shrink: 0;
}

.tabs {
  flex-shrink: 0;
  margin-top: 8px;

  &.open {
    border-bottom: 1px solid var(--schemes-outline-variant);
  }
}

.pane-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 0 8px 16px;
}

.sheet-enter-from,
.sheet-leave-to {
  transform: translateY(100%);
}
</style>
