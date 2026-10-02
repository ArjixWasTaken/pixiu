<template>
  <footer
    ref="root"
    :class="{ mobile: isMobile }"
    class="app-footer"
    data-vue="AppFooter"
    @contextmenu.prevent="requestContextMenu"
    @mousemove="showControls"
  >
    <audio id="audio-player" class="hidden" crossorigin="anonymous" />

    <div class="fullscreen-backdrop hidden" />

    <!-- Fullscreen: what is playing, large, above the controls. -->
    <div v-if="isFullscreen && stage" class="fullscreen-stage" data-testid="fullscreen-stage">
      <img :src="stage.cover" alt="" class="fullscreen-cover" />
      <div class="fullscreen-caption">
        <p class="m3-display-small truncate">{{ stage.title }}</p>
        <p v-if="stage.subtitle" class="m3-title-large truncate fullscreen-subtitle">{{ stage.subtitle }}</p>
      </div>
    </div>

    <MiniPlayer v-if="isMobile" />

    <div v-else class="wrapper">
      <SongInfo />
      <PlaybackControls />
      <ExtraControls />
    </div>

    <Transition>
      <UpNext v-show="showingUpNext" :playable="nextPlayable" class="up-next" />
    </Transition>
  </footer>

  <NowPlayingSheet v-if="isMobile" />
</template>

<script lang="ts" setup>
import { useThrottleFn, watchThrottled } from '@vueuse/core'
import { computed, nextTick, ref, watch } from 'vue'
import { useFullscreen } from '@vueuse/core'
import { eventBus } from '@/utils/eventBus'
import { isAudioContextSupported } from '@/utils/supports'
import { defineAsyncComponent, requireInjection } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { useBranding } from '@/composables/useBranding'
import { CurrentStreamableKey } from '@/config/symbols'
import { useArtistStore } from '@/stores/artistStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { audioService } from '@/services/audioService'
import { playback } from '@/services/playbackManager'
import { useContextMenu } from '@/composables/useContextMenu'
import { useViewport } from '@/composables/useViewport'
import { volumeManager } from '@/services/volumeManager'

import ExtraControls from '@/components/layout/app-footer/FooterExtraControls.vue'
import PlaybackControls from '@/components/layout/app-footer/FooterPlaybackControls.vue'
import MiniPlayer from '@/components/layout/app-footer/MiniPlayer.vue'
import NowPlayingSheet from '@/components/layout/now-playing/NowPlayingSheet.vue'

const artistStore = useArtistStore()
const preferenceStore = usePreferenceStore()

const SongInfo = defineAsyncComponent(() => import('@/components/layout/app-footer/FooterPlayableInfo.vue'))
const UpNext = defineAsyncComponent(() => import('@/components/layout/app-footer/UpNext.vue'))
const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const currentStreamable = requireInjection(CurrentStreamableKey, ref())
const { isMobile } = useViewport()
let hideControlsTimeout: number

const root = ref<HTMLElement>()
const artist = ref<Artist>()
const nextPlayable = ref<Playable | null>(null)

const { isFullscreen, toggle: toggleFullscreen } = useFullscreen(root)
const { openContextMenu } = useContextMenu()

const showingUpNext = computed(() => nextPlayable.value && isFullscreen.value)

const requestContextMenu = (event: MouseEvent) => {
  if (document.fullscreenElement || !currentStreamable.value) {
    return
  }

  openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [currentStreamable.value] })
}

watch(currentStreamable, async streamable => {
  if (!streamable) {
    return
  }

  artist.value = await artistStore.resolve(streamable.artist_id)
})

const { cover: defaultCover } = useBranding()

/** What fullscreen shows large: the cover, the title, who and from what. */
const stage = computed(() => {
  const streamable = currentStreamable.value
  if (!streamable) {
    return null
  }
  return {
    cover: streamable.album_cover || defaultCover,
    title: streamable.title,
    // A single's song stands alone: no album to name.
    subtitle: [streamable.artist_name, streamable.is_single ? null : streamable.album_name].filter(Boolean).join(' · '),
  }
})

const appBackgroundImage = computed(() => {
  if (!currentStreamable.value) {
    return 'none'
  }

  const src = artist.value?.image ?? currentStreamable.value.album_cover
  return src ? `url(${src})` : 'none'
})

const initPlaybackRelatedServices = async () => {
  const audioElement = document.querySelector<HTMLMediaElement>('#audio-player')

  if (!audioElement) {
    await nextTick()
    await initPlaybackRelatedServices()
    return
  }

  const playbackService = playback()

  // If audio context is supported, initialize the audio service which handles audio processing (equalizer, etc.)
  if (isAudioContextSupported) {
    audioService.init(playbackService.media)
  }
}

watch(
  () => preferenceStore.initialized,
  async initialized => {
    if (!initialized) {
      return
    }

    volumeManager.init(null, preferenceStore.volume)
    await initPlaybackRelatedServices()
  },
  { immediate: true },
)

// Volume changes come often: save them at most once a second.
watchThrottled(volumeManager.volume, volume => (preferenceStore.volume = volume), { throttle: 1_000 })

const setupControlHidingTimer = () => {
  hideControlsTimeout = window.setTimeout(() => root.value?.classList.add('hide-controls'), 5000)
}

const showControls = useThrottleFn(() => {
  if (!document.fullscreenElement) {
    return
  }

  root.value?.classList.remove('hide-controls')
  window.clearTimeout(hideControlsTimeout)
  setupControlHidingTimer()
}, 100)

watch(isFullscreen, fullscreen => {
  if (fullscreen) {
    setupControlHidingTimer()
    root.value?.classList.remove('hide-controls')
  } else {
    window.clearTimeout(hideControlsTimeout)
  }
})

// The browser may refuse (no user gesture, a policy): nothing to do then.
eventBus.on('FULLSCREEN_TOGGLE', () => toggleFullscreen().catch(logger.warn))
eventBus.on('UP_NEXT', next => (nextPlayable.value = next))
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.v-enter-active,
.v-leave-active {
  transition: opacity 2s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
}

footer {
  position: relative;
  z-index: 20;
  flex-shrink: 0;

  &:not(.mobile) {
    min-height: var(--m3-player-height);
    border-top: 1px solid var(--schemes-outline-variant);
    background: var(--schemes-surface-container);
  }

  &.mobile {
    padding-top: 4px;
  }

  .wrapper {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: var(--m3-player-height);
    padding: 12px 16px;

    @media (pointer: fine) and (min-width: 769px) {
      padding: 8px 12px;
    }
  }

  .fullscreen-backdrop {
    background-color: #1d1d1d;
    background-image: v-bind(appBackgroundImage);
  }

  &:fullscreen {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 32px;
    margin: 0;
    padding: 5vh 5vw 32px;
    border-radius: 0;
    background: #000;
    color: #fff;

    &.hide-controls :not(.fullscreen-backdrop, .fullscreen-stage, .fullscreen-stage *, .up-next, .up-next *) {
      transition: opacity 2s ease-in-out !important; /* overriding all children's custom transition, if any */
      @apply opacity-0;
    }

    .wrapper {
      @apply z-[3];

      border-radius: 28px;
      background: color-mix(in srgb, #000 45%, transparent);
      backdrop-filter: blur(12px);
    }

    /* Keeps the controls readable over any backdrop. */
    &::after {
      background-image: linear-gradient(0deg, rgb(0 0 0 / 70%) 0%, transparent 40vh);
      content: '';
      @apply absolute w-full h-full top-0 left-0 z-1 pointer-events-none;
    }

    .fullscreen-backdrop {
      @apply block absolute top-0 left-0 w-full h-full z-0 bg-cover bg-no-repeat bg-center;

      filter: blur(48px) brightness(0.45) saturate(1.2);
      transform: scale(1.15);
    }

    .fullscreen-stage {
      position: relative;
      z-index: 2;
      flex: 1;
      min-height: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 32px;
    }

    .fullscreen-cover {
      width: min(52vh, 80vw);
      aspect-ratio: 1 / 1;
      object-fit: cover;
      border-radius: 28px;
      box-shadow: 0 24px 64px rgb(0 0 0 / 50%);
    }

    .fullscreen-caption {
      max-width: min(80vw, 960px);
      text-align: center;
    }

    .fullscreen-subtitle {
      opacity: 0.75;
    }
  }
}
</style>
