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

    <MiniPlayer v-if="isMobile" />

    <div v-else class="wrapper">
      <RadioStationInfo v-if="isRadio" />
      <SongInfo v-else />
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
import { isEpisode, isRadioStation, isSong } from '@/utils/typeGuards'
import { isAudioContextSupported } from '@/utils/supports'
import { defineAsyncComponent, requireInjection } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { artistStore } from '@/stores/artistStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { audioService } from '@/services/audioService'
import { playback } from '@/services/playbackManager'
import { useContextMenu } from '@/composables/useContextMenu'
import { useViewport } from '@/composables/useViewport'
import { volumeManager } from '@/services/volumeManager'
import { socketService } from '@/services/socketService'

import ExtraControls from '@/components/layout/app-footer/FooterExtraControls.vue'
import PlaybackControls from '@/components/layout/app-footer/FooterPlaybackControls.vue'
import MiniPlayer from '@/components/layout/app-footer/MiniPlayer.vue'
import NowPlayingSheet from '@/components/layout/now-playing/NowPlayingSheet.vue'

const SongInfo = defineAsyncComponent(() => import('@/components/layout/app-footer/FooterPlayableInfo.vue'))
const RadioStationInfo = defineAsyncComponent(() => import('@/components/layout/app-footer/FooterRadioStationInfo.vue'))
const UpNext = defineAsyncComponent(() => import('@/components/layout/app-footer/UpNext.vue'))
const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))
const RadioStationContextMenu = defineAsyncComponent(() => import('@/components/radio/RadioStationContextMenu.vue'))

const currentStreamable = requireInjection(CurrentStreamableKey, ref())
const { isMobile } = useViewport()
let hideControlsTimeout: number

const root = ref<HTMLElement>()
const artist = ref<Artist>()
const nextPlayable = ref<Playable | null>(null)

const { isFullscreen, toggle: toggleFullscreen } = useFullscreen(root)
const { openContextMenu } = useContextMenu()

const showingUpNext = computed(() => nextPlayable.value && isFullscreen.value)
const isRadio = computed(() => currentStreamable.value && isRadioStation(currentStreamable.value))

const requestContextMenu = (event: MouseEvent) => {
  if (document.fullscreenElement || !currentStreamable.value) {
    return
  }

  if (isRadio.value) {
    openContextMenu<'RADIO_STATION'>(RadioStationContextMenu, event, {
      station: currentStreamable.value as RadioStation,
    })
  } else {
    openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, {
      playables: [currentStreamable.value as Playable],
    })
  }
}

watch(currentStreamable, async streamable => {
  if (!streamable) {
    return
  }

  if (isSong(streamable)) {
    artist.value = await artistStore.resolve(streamable.artist_id)
  }
})

const appBackgroundImage = computed(() => {
  if (!currentStreamable.value) {
    return 'none'
  }

  let src: string | null = null

  if (isSong(currentStreamable.value)) {
    src = artist.value?.image ?? currentStreamable.value.album_cover
  } else if (isEpisode(currentStreamable.value)) {
    src = currentStreamable.value.episode_image
  } else if (isRadio.value) {
    src = (currentStreamable.value as RadioStation).logo
  }

  return src ? `url(${src})` : 'none'
})

const initPlaybackRelatedServices = async () => {
  const audioElement = document.querySelector<HTMLMediaElement>('#audio-player')

  if (!audioElement) {
    await nextTick()
    await initPlaybackRelatedServices()
    return
  }

  // Defaults to the queue playback over radio playback.
  const playbackService = playback()

  // If audio context is supported, initialize the audio service which handles audio processing (equalizer, etc.)
  if (isAudioContextSupported) {
    audioService.init(playbackService.media)
  }
}

watch(
  preferenceStore.initialized,
  async initialized => {
    if (!initialized) {
      return
    }

    volumeManager.init(null, preferenceStore.volume)
    await initPlaybackRelatedServices()
  },
  { immediate: true },
)

// Volume changes come often: save and broadcast them at most once a second.
watchThrottled(
  volumeManager.volume,
  volume => {
    preferenceStore.volume = volume
    socketService.broadcast('SOCKET_VOLUME_CHANGED', volume)
  },
  { throttle: 1_000 },
)

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

eventBus.on('FULLSCREEN_TOGGLE', () => toggleFullscreen()).on('UP_NEXT', next => (nextPlayable.value = next))
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
    margin: 12px;
    min-height: 88px;
    border-radius: 28px;
    background: var(--schemes-surface-container-high);
  }

  &.mobile {
    padding-top: 4px;
  }

  .wrapper {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 88px;
    padding: 12px 16px;
  }

  .fullscreen-backdrop {
    background-color: #1d1d1d;
    background-image: v-bind(appBackgroundImage);
  }

  &:fullscreen {
    padding: calc(100vh - 9rem) 5vw 0;
    @apply bg-none;

    &.hide-controls :not(.fullscreen-backdrop, .up-next, .up-next *) {
      transition: opacity 2s ease-in-out !important; /* overriding all children's custom transition, if any */
      @apply opacity-0;
    }

    &.hide-controls::after {
      transition: opacity 2s ease-in-out !important;
      @apply opacity-0;
    }

    .wrapper {
      @apply z-[3];
    }

    &::before {
      @apply bg-black bg-repeat absolute top-0 left-0 opacity-50 z-1 pointer-events-none -m-[20rem];
      content: '';
      background-image:
        linear-gradient(135deg, #111 25%, transparent 25%), linear-gradient(225deg, #111 25%, transparent 25%),
        linear-gradient(45deg, #111 25%, transparent 25%), linear-gradient(315deg, #111 25%, rgba(255, 255, 255, 0) 25%);
      background-position:
        6px 0,
        6px 0,
        0 0,
        0 0;
      background-size: 6px 6px;
      width: calc(100% + 40rem);
      height: calc(100% + 40rem);
      transform: rotate(10deg);
    }

    &::after {
      background-image: linear-gradient(0deg, var(--color-bg) 0%, rgba(255, 255, 255, 0) 30vh);
      content: '';
      @apply absolute w-full h-full top-0 left-0 z-1 pointer-events-none;
    }

    .fullscreen-backdrop {
      @apply saturate-[0.2] block absolute top-0 left-0 w-full h-full z-0 bg-cover bg-no-repeat bg-top;
    }
  }
}
</style>
