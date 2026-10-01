<template>
  <M3IconButton
    :icon="mode === 'REPEAT_ONE' ? 'repeat_one' : 'repeat'"
    :label="`Repeat: ${readableMode}`"
    :selected="mode !== 'NO_REPEAT'"
    data-testid="repeat-mode-switch"
    @click.prevent="changeMode"
  />
</template>

<script lang="ts" setup>
import { computed, toRef } from 'vue'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { playback } from '@/services/playbackManager'

import M3IconButton from '@/components/m3/M3IconButton.vue'

const preferenceStore = usePreferenceStore()

const mode = toRef(preferenceStore.state, 'repeat_mode')

/** What a press on it changes: the label says where it is now. */
const readableMode = computed(() => ({ NO_REPEAT: 'off', REPEAT_ALL: 'all', REPEAT_ONE: 'one' })[mode.value])

const changeMode = () => playback().rotateRepeatMode()
</script>
