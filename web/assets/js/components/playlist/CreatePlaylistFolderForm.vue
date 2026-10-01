<template>
  <form class="md:w-[420px] min-w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>New playlist folder</h1>
    </header>

    <main class="pt-2">
      <M3TextField v-model="data.name" v-koel-focus label="Name" name="name" required />
    </main>

    <footer>
      <M3Button variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { playlistFolderStore } from '@/stores/playlistFolderStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const emit = defineEmits<{ (e: 'close'): void }>()
const props = withDefaults(defineProps<{ parent?: PlaylistFolder | null }>(), {
  parent: null,
})

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const close = () => emit('close')

const { data, isPristine, handleSubmit } = useForm<Pick<PlaylistFolder, 'name'>>({
  initialValues: {
    name: '',
  },
  onSubmit: async ({ name }) =>
    props.parent ? await playlistFolderStore.store(name, props.parent) : await playlistFolderStore.store(name),
  onSuccess: (folder: PlaylistFolder) => {
    close()
    toastSuccess(`Playlist folder "${folder.name}" created.`)
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>
