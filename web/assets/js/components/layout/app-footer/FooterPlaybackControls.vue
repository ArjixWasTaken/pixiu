<template>
  <div class="playback-controls" data-vue="FooterPlaybackControls">
    <div class="buttons">
      <M3IconButton
        :disabled="isRadio"
        fill
        icon="skip_previous"
        label="Play previous in queue"
        @click.prevent="playPrev"
      />
      <PlayButton />
      <M3IconButton :disabled="isRadio" fill icon="skip_next" label="Play next in queue" @click.prevent="playNext" />
      <RepeatModeSwitch :class="isRadio && 'pointer-events-none opacity-30'" />
    </div>
    <AudioPlayer v-show="streamable" :class="isRadio && 'pointer-events-none'" />
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { playback } from '@/services/playbackManager'

import AudioPlayer from '@/components/layout/app-footer/AudioPlayer.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlayButton from '@/components/ui/FooterPlayButton.vue'
import RepeatModeSwitch from '@/components/ui/RepeatModeSwitch.vue'

const streamable = requireInjection(CurrentStreamableKey, ref())

const isRadio = computed(() => streamable.value?.type === 'radio-stations')

const playPrev = async () => isRadio.value || (await playback().playPrev())
const playNext = async () => isRadio.value || (await playback().playNext())
</script>

<style scoped>
.playback-controls {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
