<template>
  <form class="md:w-[560px] w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>
        New playlist
        <span v-if="playables.length" data-testid="from-playables">from {{ pluralize(playables, 'song') }}</span>
      </h1>
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
import type { CreatePlaylistData } from '@/stores/playlistStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { pluralize } from '@/utils/formatters'
import { useRouter } from '@/composables/useRouter'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import PlaylistDetails from '@/components/playlist/PlaylistDetails.vue'

const playlistStore = usePlaylistStore()

const props = withDefaults(defineProps<{ playables?: Playable[]; folder?: PlaylistFolder | null }>(), {
  playables: () => [],
})

const emit = defineEmits<{ (e: 'close'): void }>()

const { playables, folder: targetFolder } = props

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { go, url } = useRouter()

const close = () => emit('close')

const { data, loading, isPristine, handleSubmit } = useForm<CreatePlaylistData>({
  initialValues: {
    name: '',
    description: '',
    folder_id: targetFolder?.id ?? null,
    folder_name: null,
    cover: null,
  },
  onSubmit: async data => await playlistStore.store(data, playables),
  onSuccess: (playlist: Playlist) => {
    close()
    toastSuccess(`Playlist "${playlist.name}" created.`)
    go(url('playlists.show', { id: playlist.id }))
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>
