<template>
  <div ref="root">
    <M3TextField
      v-if="entering"
      v-model="inputName"
      aria-label="New folder name"
      label="New folder"
      placeholder="Folder name"
      @keydown.enter.prevent="confirm"
      @keydown.esc.stop.prevent="cancel"
    >
      <template #trailing>
        <span class="flex -mr-3">
          <M3IconButton icon="check" label="Create" @click="confirm" />
          <M3IconButton icon="close" label="Cancel" @click="cancel" />
        </span>
      </template>
    </M3TextField>
    <M3Select v-else v-model="selected" label="Folder" @update:model-value="onSelectChange">
      <option :value="null">None</option>
      <option v-for="folder in folders" :key="folder.id" :value="folder.id">
        {{ playlistFolderStore.pathFor(folder) }}
      </option>
      <option v-if="folderName" :value="PENDING_FOLDER">{{ folderName }} (new)</option>
      <option :value="NEW_FOLDER">New folder…</option>
    </M3Select>
  </div>
</template>

<script lang="ts" setup>
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'
import { orderBy } from 'lodash-es'
import { playlistFolderStore } from '@/stores/playlistFolderStore'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Select from '@/components/m3/M3Select.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const NEW_FOLDER = '__new__'
const PENDING_FOLDER = '__pending__'

const folderId = defineModel<PlaylistFolder['id'] | null | undefined>('folderId', { required: true })
const folderName = defineModel<string | null>('folderName', { default: null })

const folders = computed(() =>
  orderBy(playlistFolderStore.state.folders, folder => playlistFolderStore.pathFor(folder)),
)
const entering = ref(false)
const inputName = ref('')
const root = useTemplateRef('root')
const selected = ref(folderName.value ? PENDING_FOLDER : folderId.value)

watch([folderId, folderName], ([id, name]) => {
  selected.value = name ? PENDING_FOLDER : id
})

const onSelectChange = () => {
  if (selected.value === NEW_FOLDER || selected.value === PENDING_FOLDER) {
    entering.value = true
    inputName.value = folderName.value ?? ''
    nextTick(() => root.value?.querySelector('input')?.focus())
  } else {
    folderId.value = selected.value
    folderName.value = null
  }
}

const confirm = () => {
  const name = inputName.value.trim()

  if (!name) {
    return
  }

  folderName.value = name
  folderId.value = null
  selected.value = PENDING_FOLDER
  entering.value = false
}

const cancel = () => {
  entering.value = false
  inputName.value = ''
  selected.value = folderName.value ? PENDING_FOLDER : folderId.value
}
</script>
