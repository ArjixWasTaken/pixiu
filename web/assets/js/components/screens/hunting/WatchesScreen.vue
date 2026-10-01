<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Watches</ScreenHeader>
    </template>

    <p class="m3-body-large text-(--schemes-on-surface-variant) pt-1 mb-5 max-w-[64ch]">
      píxiū downloads what watched playlists, your liked music and artists’ new releases add, and mirrors watched
      playlists as read-only playlists.
    </p>

    <AddWatchForm class="mb-5" @added="refetch" />

    <ScreenEmptyState v-if="loaded && !watches.length">
      <template #icon>
        <M3Icon :size="64" name="visibility" />
      </template>
      Nothing watched yet
      <span class="secondary block">Paste a playlist or artist link from music.youtube.com above.</span>
    </ScreenEmptyState>

    <ul v-else class="flex flex-col gap-3" data-vue="WatchesScreen">
      <WatchRow v-for="watch in watches" :key="watch.id" :watch @remove="remove(watch)" @sync="sync(watch)" />
    </ul>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { computed, watch as watchRef } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Watch } from '@/services/huntingService'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import AddWatchForm from '@/components/screens/hunting/AddWatchForm.vue'
import WatchRow from '@/components/screens/hunting/WatchRow.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

// Fetched again whenever the server says the job board changed (see huntingStore).
const {
  data,
  isSuccess: loaded,
  error,
  refetch,
} = useQuery({ queryKey: ['hunting', 'watches'], queryFn: () => huntingService.watches() })
watchRef(error, error => error && handleHttpError(error))

const watches = computed(() => data.value ?? [])

const sync = async (watch: Watch) => {
  try {
    await huntingService.syncWatch(watch.id)
    toastSuccess(`Syncing “${watch.name}”.`)
    await refetch()
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
    await refetch()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}
</script>
