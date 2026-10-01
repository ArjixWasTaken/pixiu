<template>
  <M3IconButton
    v-if="swReady"
    :class="state"
    :disabled="state === 'caching'"
    :fill="state === 'cached'"
    :icon
    :icon-size="20"
    :label
    class="offline-button"
    @click.prevent.stop="toggle"
  >
    <M3ProgressIndicator v-if="state === 'caching'" :size="20" :value="progress || undefined" variant="circular" />
  </M3IconButton>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

/**
 * Makes a song available offline, or no longer: a down arrow in a circle,
 * then its progress, then a filled check. Shown only where a service worker
 * keeps songs (not under the dev server, say).
 */
const props = defineProps<{ playable: Playable }>()
const { playable } = toRefs(props)

const {
  swReady,
  isCached,
  isCaching,
  getCachingProgress,
  hasCachingError,
  getCachingError,
  makeAvailableOffline,
  removeOfflineCache,
} = useOfflinePlayback()

const state = computed(() => {
  if (isCaching(playable.value)) {
    return 'caching'
  }

  if (isCached(playable.value)) {
    return 'cached'
  }

  return hasCachingError(playable.value) ? 'failed' : 'available'
})

const progress = computed(() => getCachingProgress(playable.value))

const icon = computed(
  () =>
    ({ available: 'arrow_circle_down', caching: 'arrow_circle_down', cached: 'check_circle', failed: 'error' })[
      state.value
    ],
)

const label = computed(
  () =>
    ({
      available: 'Make available offline',
      caching: 'Making available offline…',
      cached: 'Remove offline copy',
      failed: `Couldn’t make it available offline (${getCachingError(playable.value)}). Try again`,
    })[state.value],
)

const toggle = () =>
  state.value === 'cached' ? removeOfflineCache(playable.value) : makeAvailableOffline(playable.value)
</script>

<style scoped>
.cached {
  color: var(--schemes-primary);
}

.failed {
  color: var(--schemes-error);
}
</style>
