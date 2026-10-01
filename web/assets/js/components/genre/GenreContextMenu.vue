<template>
  <ul role="none">
    <MenuItem @click="play">Play</MenuItem>
    <MenuItem @click="shuffle">Shuffle</MenuItem>
    <MenuItem @click="queue">Add to queue</MenuItem>
  </ul>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'
import { useContextMenu } from '@/composables/useContextMenu'
import { playback } from '@/services/playbackManager'
import { usePlayableStore } from '@/stores/playableStore'
import { useRouter } from '@/composables/useRouter'
import { useQueueStore } from '@/stores/queueStore'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { pluralize } from '@/utils/formatters'

const playableStore = usePlayableStore()
const queueStore = useQueueStore()

const props = defineProps<{ genre: Genre }>()
const { genre } = toRefs(props)

const { MenuItem, trigger } = useContextMenu()
const { toastSuccess } = useMessageToaster()
const { go } = useRouter()

const play = () =>
  trigger(async () => {
    go('queue')
    await playback().queueAndPlay(await playableStore.fetchSongsByGenre(genre.value))
  })

const shuffle = () =>
  trigger(async () => {
    go('queue')
    await playback().queueAndPlay(await playableStore.fetchSongsByGenre(genre.value, true))
  })

const queue = () =>
  trigger(async () => {
    const songs = await playableStore.fetchSongsByGenre(genre.value)
    queueStore.queue(songs)
    toastSuccess(`${pluralize(songs, 'song')} added to queue.`)
  })
</script>
