<template>
  <form class="md:w-[420px] min-w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>Edit playlist folder</h1>
    </header>

    <main class="flex flex-col gap-4 pt-2">
      <M3TextField v-model="data.name" v-koel-focus label="Name" name="name" required />
      <M3Select v-model="data.parent_id" label="Inside">
        <option :value="null">No folder</option>
        <option v-for="parent in parentFolders" :key="parent.id" :value="parent.id">
          {{ playlistFolderStore.pathFor(parent) }}
        </option>
      </M3Select>
    </main>

    <footer>
      <M3Button variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { orderBy, pick } from 'lodash-es'
import { computed } from 'vue'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3Select from '@/components/m3/M3Select.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const playlistFolderStore = usePlaylistFolderStore()

const props = defineProps<{ folder: PlaylistFolder }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { folder } = props

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const parentFolders = computed(() => {
  const unavailableParentIds = new Set([
    folder.id,
    ...playlistFolderStore.descendantsOf(folder).map(descendant => descendant.id),
  ])

  return orderBy(
    playlistFolderStore.state.folders.filter(candidate => !unavailableParentIds.has(candidate.id)),
    candidate => playlistFolderStore.pathFor(candidate),
  )
})

const close = () => emit('close')

const { data, isPristine, handleSubmit } = useForm<Pick<PlaylistFolder, 'name' | 'parent_id'>>({
  initialValues: pick(folder, 'name', 'parent_id'),
  onSubmit: async changes => await playlistFolderStore.update(folder, changes),
  onSuccess: () => {
    toastSuccess('Playlist folder updated.')
    close()
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?', { action: 'Discard' }))) {
    close()
  }
}
</script>
