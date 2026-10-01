<template>
  <form
    class="smart-playlist-form"
    @invalid.capture="onInvalid"
    @keydown.esc="maybeClose"
    @submit.prevent="handleSubmit"
  >
    <header>
      <h1>New smart playlist</h1>
    </header>

    <main>
      <SmartPlaylistEditor
        v-model:description="data.description"
        v-model:folder-id="data.folder_id"
        v-model:folder-name="data.folder_name"
        v-model:name="data.name"
        v-model:rule-groups="ruleGroups"
        v-model:tab="currentTab"
        :tabs
      />
    </main>

    <footer>
      <M3Button type="button" variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button :disabled="loading" type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { isEqual } from 'lodash-es'
import type { CreatePlaylistData } from '@/stores/playlistStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useSmartPlaylistForm } from '@/composables/useSmartPlaylistForm'
import { useRouter } from '@/composables/useRouter'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import SmartPlaylistEditor from '@/components/playlist/smart-playlist/SmartPlaylistEditor.vue'

const playlistStore = usePlaylistStore()

const props = defineProps<{ folder?: PlaylistFolder | null }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { go, url } = useRouter()

// A new playlist starts with a rule to fill in.
const { tabs, currentTab, ruleGroups, rulesChanged, onInvalid } = useSmartPlaylistForm([
  playlistStore.createEmptySmartPlaylistRuleGroup(),
])

const close = () => emit('close')

const { data, loading, isPristine, handleSubmit } = useForm<CreatePlaylistData>({
  initialValues: {
    name: '',
    description: '',
    folder_id: props.folder?.id || null,
    folder_name: null,
    cover: null,
  },
  isPristine: (original, current) => isEqual(original, current) && !rulesChanged(),
  onSubmit: async data => await playlistStore.store({ ...data, rules: ruleGroups.value }),
  onSuccess: (playlist: Playlist) => {
    toastSuccess(`Playlist "${playlist.name}" created.`)
    close()
    go(url('playlists.show', { id: playlist.id }))
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>

<style scoped>
.smart-playlist-form {
  width: min(640px, 100vw);
}
</style>
