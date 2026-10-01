<template>
  <button
    :style="{ backgroundImage: `url(${defaultCover})` }"
    :title
    class="song-thumbnail"
    type="button"
    @click.prevent.stop="emit('clicked')"
  >
    <img v-if="src" :src alt="Cover image" loading="lazy" />
    <span v-if="current" class="now">
      <template v-if="playable.playback_state === 'Playing'">
        <span
          v-for="i in 3"
          :key="i"
          :style="{ animationDuration: `${0.65 + i * 0.25}s`, animationDelay: `${i * -0.3}s` }"
          class="bar"
        />
      </template>
      <M3Icon v-else name="pause" />
    </span>
    <span v-else class="hover">
      <M3Icon fill name="play_arrow" />
    </span>
  </button>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { useBranding } from '@/composables/useBranding'

import M3Icon from '@/components/m3/M3Icon.vue'

const props = defineProps<{ playable: Playable }>()
const emit = defineEmits<{ (e: 'clicked'): void }>()

const { playable } = toRefs(props)

const { cover: defaultCover } = useBranding()

const src = computed(() => playable.value.album_cover)
const current = computed(() => ['Playing', 'Paused'].includes(playable.value.playback_state!))

const title = computed(() => {
  if (playable.value.playback_state === 'Playing') {
    return 'Pause'
  }

  if (playable.value.playback_state === 'Paused') {
    return 'Resume'
  }

  return 'Play'
})
</script>

<style scoped>
.song-thumbnail {
  position: relative;
  display: block;
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 8px;
  background-size: cover;
  background-position: center;

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.now,
.hover {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  background: color-mix(in srgb, var(--schemes-scrim) 45%, transparent);
  color: var(--schemes-primary);
}

.hover {
  opacity: 0;
  color: #fff;
  transition: opacity 150ms linear;

  .song-thumbnail:hover &,
  :global(.song-item:hover) & {
    opacity: 1;
  }

  @media (hover: none) {
    display: none;
  }
}

.bar {
  width: 4px;
  height: 18px;
  border-radius: 2px;
  background: var(--schemes-primary);
  transform-origin: bottom;
  animation: m3-equalizer ease-in-out infinite;
}
</style>
