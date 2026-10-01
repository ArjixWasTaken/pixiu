<template>
  <form class="md:w-[560px] w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>Edit Playlist</h1>
    </header>

    <main class="pt-2">
      <PlaylistDetails
        v-model:description="data.description"
        v-model:folder-id="data.folder_id"
        v-model:folder-name="data.folder_name"
        v-model:name="data.name"
      />
    </main>

    <footer>
      <M3Button type="button" variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button :disabled="loading" type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { pick } from 'lodash-es'
import { toRaw } from 'vue'

import type { UpdatePlaylistData } from '@/stores/playlistStore'
import { playlistStore } from '@/stores/playlistStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import PlaylistDetails from '@/components/playlist/PlaylistDetails.vue'

const props = defineProps<{ playlist: Playlist }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { playlist } = props

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const close = () => emit('close')

const { data, loading, isPristine, handleSubmit } = useForm<UpdatePlaylistData>({
  initialValues: { ...pick(playlist, 'name', 'folder_id', 'description', 'cover'), folder_name: null },
  onSubmit: async data => {
    const formData = structuredClone(toRaw(data))

    if (formData.cover === playlist.cover) {
      delete formData.cover
    }

    await playlistStore.update(playlist, formData)
  },
  onSuccess: () => {
    toastSuccess('Playlist updated.')
    close()
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>

<style lang="postcss" scoped>
form {
  min-width: 100%;
}

label.folder {
  flex: 0.6;
}
</style>
