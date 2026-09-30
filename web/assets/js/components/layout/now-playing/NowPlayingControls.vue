<template>
  <div class="now-playing-controls">
    <M3IconButton fill icon="skip_previous" label="Play previous in queue" size="m" @click="playPrev" />
    <PlayButton shape="square" size="m" />
    <M3IconButton fill icon="skip_next" label="Play next in queue" size="m" @click="playNext" />
    <RepeatModeSwitch />
  </div>
</template>

<script lang="ts" setup>
import { playback } from '@/services/playbackManager'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlayButton from '@/components/ui/FooterPlayButton.vue'
import RepeatModeSwitch from '@/components/ui/RepeatModeSwitch.vue'

const playPrev = () => playback().playPrev()
const playNext = () => playback().playNext()
</script>

<style scoped>
.now-playing-controls {
  display: flex;
  align-items: center;
  justify-content: space-around;
  padding-bottom: 4px;
}
</style>
