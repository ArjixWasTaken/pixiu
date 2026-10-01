<template>
  <M3IconButton
    :icon="mode === 'REPEAT_ONE' ? 'repeat_one' : 'repeat'"
    :label="`Change repeat mode (current: ${readableMode})`"
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

const readableMode = computed(() =>
  mode.value
    .split('_')
    .map(part => part[0].toUpperCase() + part.substring(1).toLowerCase())
    .join(' '),
)

const changeMode = () => playback().rotateRepeatMode()
</script>
