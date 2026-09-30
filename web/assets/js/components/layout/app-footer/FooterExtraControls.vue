<template>
  <div class="extra-controls" data-vue="FooterExtraControls">
    <M3IconButton v-if="useEqualizer" icon="equalizer" label="Show equalizer" @click.prevent="showEqualizer" />
    <VolumeSlider />
    <M3IconButton
      v-if="isFullscreenSupported()"
      :icon="isFullscreen ? 'fullscreen_exit' : 'fullscreen'"
      :label="fullscreenButtonTitle"
      @click.prevent="toggleFullscreen"
    />
    <M3IconButton
      :icon="nowPlaying.open.value ? 'keyboard_arrow_down' : 'keyboard_arrow_up'"
      :label="nowPlaying.open.value ? 'Collapse player' : 'Expand player'"
      :selected="nowPlaying.open.value"
      data-testid="now-playing-toggle"
      variant="tonal"
      @click.prevent="nowPlaying.toggle"
    />
  </div>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { isFullscreenSupported, isAudioContextSupported as useEqualizer } from '@/utils/supports'
import { defineAsyncComponent } from '@/utils/helpers'
import { useModal } from '@/composables/useModal'
import { useNowPlaying } from '@/composables/useNowPlaying'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import VolumeSlider from '@/components/ui/VolumeSlider.vue'

const Equalizer = defineAsyncComponent(() => import('@/components/ui/equalizer/Equalizer.vue'))
const { openModal } = useModal()
const nowPlaying = useNowPlaying()

const isFullscreen = ref(false)
const fullscreenButtonTitle = computed(() => (isFullscreen.value ? 'Exit fullscreen mode' : 'Enter fullscreen mode'))

const showEqualizer = () => openModal<'EQUALIZER'>(Equalizer)
const toggleFullscreen = () => eventBus.emit('FULLSCREEN_TOGGLE')

const onFullscreenChange = () => (isFullscreen.value = Boolean(document.fullscreenElement))

onMounted(() => document.addEventListener('fullscreenchange', onFullscreenChange))
onBeforeUnmount(() => document.removeEventListener('fullscreenchange', onFullscreenChange))
</script>

<style scoped>
.extra-controls {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  width: 300px;
  flex-shrink: 0;
}
</style>
