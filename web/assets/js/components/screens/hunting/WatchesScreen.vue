<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Watches</ScreenHeader>
    </template>

    <p class="text-k-fg-70 mb-4 max-w-[64ch]">
      píxiū downloads what watched playlists, your liked music and artists’ new releases add, and mirrors watched
      playlists as read-only playlists.
    </p>

    <AddWatchForm class="mb-8" @added="fetchWatches" />

    <ScreenEmptyState v-if="loaded && !watches.length">
      <template #icon>
        <Icon :icon="faEye" />
      </template>
      Nothing watched yet
      <span class="secondary block">Paste a playlist or artist link from music.youtube.com above.</span>
    </ScreenEmptyState>

    <ul v-else class="flex flex-col gap-3">
      <WatchRow v-for="watch in watches" :key="watch.id" :watch @remove="remove(watch)" @sync="sync(watch)" />
    </ul>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faEye } from '@fortawesome/free-solid-svg-icons'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Watch } from '@/services/huntingService'
import { eventBus } from '@/utils/eventBus'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import AddWatchForm from '@/components/screens/hunting/AddWatchForm.vue'
import WatchRow from '@/components/screens/hunting/WatchRow.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const watches = ref<Watch[]>([])
const loaded = ref(false)

const fetchWatches = async () => {
  try {
    watches.value = await huntingService.watches()
    loaded.value = true
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const sync = async (watch: Watch) => {
  try {
    await huntingService.syncWatch(watch.id)
    toastSuccess(`Syncing “${watch.name}”.`)
    await fetchWatches()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const remove = async (watch: Watch) => {
  const confirmed = await showConfirmDialog(
    `Stop watching “${watch.name}”? What it kept becomes orphans, unless something else keeps it.`,
    'Stop watching',
  )

  if (!confirmed) {
    return
  }

  try {
    await huntingService.removeWatch(watch.id)
    toastSuccess(`No longer watching “${watch.name}”.`)
    await fetchWatches()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchWatches)
eventBus.on('HUNT_JOBS_CHANGED', fetchWatches)
onBeforeUnmount(() => eventBus.off('HUNT_JOBS_CHANGED', fetchWatches))
</script>
