<template>
  <div class="playback-controls" data-vue="FooterPlaybackControls">
    <div class="buttons">
      <M3IconButton fill icon="skip_previous" label="Play previous in queue" @click.prevent="playPrev" />
      <PlayButton />
      <M3IconButton fill icon="skip_next" label="Play next in queue" @click.prevent="playNext" />
      <RepeatModeSwitch />
    </div>
    <AudioPlayer v-show="streamable" />
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { CurrentStreamableKey } from '@/config/symbols'
import { playback } from '@/services/playbackManager'

import AudioPlayer from '@/components/layout/app-footer/AudioPlayer.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlayButton from '@/components/ui/FooterPlayButton.vue'
import RepeatModeSwitch from '@/components/ui/RepeatModeSwitch.vue'

const streamable = requireInjection(CurrentStreamableKey, ref())

const playPrev = () => playback().playPrev()
const playNext = () => playback().playNext()
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
