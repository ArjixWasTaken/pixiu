<template>
  <div class="flex flex-col gap-4">
    <div class="grid md:grid-cols-2 gap-4">
      <M3TextField v-model="name" v-koel-focus label="Name" name="name" required />
      <FolderSelect v-model:folder-id="folderId" v-model:folder-name="folderName" />
    </div>
    <M3TextField v-model="description" :rows="4" label="Description" multiline name="description" />
  </div>
</template>

<script lang="ts" setup>
import M3TextField from '@/components/m3/M3TextField.vue'
import FolderSelect from '@/components/ui/form/FolderSelect.vue'

const name = defineModel<string>('name', { required: true })
const description = defineModel<string>('description', { required: true })
const folderId = defineModel<PlaylistFolder['id'] | null | undefined>('folderId', { required: true })
const folderName = defineModel<string | null>('folderName', { default: null })
</script>
