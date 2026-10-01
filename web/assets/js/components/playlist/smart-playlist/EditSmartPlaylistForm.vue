<template>
  <form
    class="smart-playlist-form"
    @invalid.capture="onInvalid"
    @keydown.esc="maybeClose"
    @submit.prevent="handleSubmit"
  >
    <header>
      <h1>Edit smart playlist</h1>
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
import { isEqual, pick } from 'lodash-es'
import { toRaw } from 'vue'
import type { UpdatePlaylistData } from '@/stores/playlistStore'
import { playlistStore } from '@/stores/playlistStore'
import { eventBus } from '@/utils/eventBus'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useSmartPlaylistForm } from '@/composables/useSmartPlaylistForm'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import SmartPlaylistEditor from '@/components/playlist/smart-playlist/SmartPlaylistEditor.vue'

const props = defineProps<{ playlist: Playlist }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { playlist } = props

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const { tabs, currentTab, ruleGroups, rulesChanged, onInvalid } = useSmartPlaylistForm(playlist.rules)

const close = () => emit('close')

const { data, loading, isPristine, handleSubmit } = useForm<UpdatePlaylistData>({
  initialValues: { ...pick(playlist, 'name', 'folder_id', 'description', 'cover'), folder_name: null },
  isPristine: (original, current) => isEqual(original, current) && !rulesChanged(),
  onSubmit: async data => {
    const formData = {
      ...structuredClone(toRaw(data)),
      rules: ruleGroups.value,
    }

    if (formData.cover === playlist.cover) {
      delete formData.cover
    }

    await playlistStore.update(playlist, formData)
  },
  onSuccess: () => {
    toastSuccess(`Playlist "${playlist.name}" updated.`)
    eventBus.emit('PLAYLIST_UPDATED', playlist)
    close()
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
