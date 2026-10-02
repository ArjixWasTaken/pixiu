<template>
  <button
    :class="{ numbered }"
    :style="numbered ? undefined : { backgroundImage: `url(${defaultCover})` }"
    :title
    class="song-thumbnail"
    type="button"
    @click.prevent.stop="emit('clicked')"
  >
    <span v-if="numbered" class="number m3-body-medium">{{ playable.track || '–' }}</span>
    <img v-else-if="src" :src alt="Cover image" class="cover" loading="lazy" />
    <PlatformBadge v-if="!numbered" :platform="playable.source_platform" />
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
import { coverOfSize } from '@/services/subsonic'

import M3Icon from '@/components/m3/M3Icon.vue'
import PlatformBadge from '@/components/ui/PlatformBadge.vue'

const props = withDefaults(
  defineProps<{
    playable: Playable
    /** The track number in place of the cover, as in an album's list. */
    numbered?: boolean
  }>(),
  { numbered: false },
)
const emit = defineEmits<{ (e: 'clicked'): void }>()

const { playable } = toRefs(props)

const { cover: defaultCover } = useBranding()

const src = computed(() => coverOfSize(playable.value.album_cover, 128))
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
  width: var(--m3-row-cover);
  height: var(--m3-row-cover);
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 6px;
  background-size: cover;
  background-position: center;

  .cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.numbered {
  background: transparent;

  .number {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--schemes-on-surface-variant);
    font-variant-numeric: tabular-nums;
  }

  .now,
  .hover {
    background: var(--schemes-surface);
  }

  .hover {
    color: var(--schemes-on-surface);
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
