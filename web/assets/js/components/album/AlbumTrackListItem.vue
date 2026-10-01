<template>
  <div
    :class="{ active, available: matchedSong }"
    :title="tooltip"
    class="track-list-item flex flex-1 gap-1"
    tabindex="0"
    @click="play"
  >
    <span class="flex-1">{{ track.title }}</span>
    <span class="w-14 text-right text-k-fg-50">{{ fmtLength }}</span>
  </div>
</template>

<script lang="ts" setup>
import type { Ref } from 'vue'
import { computed, toRefs } from 'vue'
import { playableStore } from '@/stores/playableStore'
import { requireInjection } from '@/utils/helpers'
import { secondsToHis } from '@/utils/formatters'
import { PlayablesKey } from '@/config/symbols'
import { playback } from '@/services/playbackManager'

const props = defineProps<{ album: Album; track: AlbumTrack }>()

const { track } = toRefs(props)

const songsToMatchAgainst = requireInjection<Ref<Song[]>>(PlayablesKey)

const matchedSong = computed(() => playableStore.matchSongsByTitle(track.value.title, songsToMatchAgainst.value))
const tooltip = computed(() => (matchedSong.value ? 'Click to play' : ''))
const fmtLength = computed(() => secondsToHis(track.value.length))

const active = computed(() => matchedSong.value && matchedSong.value.playback_state !== 'Stopped')

const play = () => matchedSong.value && playback().play(matchedSong.value)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.track-list-item {
  &:focus,
  &.active {
    span.title {
      @apply text-k-highlight;
    }
  }

  &.available {
    @apply cursor-pointer text-k-fg;

    &:hover {
      @apply text-k-highlight;
    }
  }
}
</style>
