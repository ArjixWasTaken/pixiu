<template>
  <form class="md:w-[560px]" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>Edit Album</h1>
    </header>

    <main class="space-y-5">
      <p class="text-sm text-k-fg-70">The files’ tags change along, and the files move to follow the file layout.</p>
      <FormRow>
        <template #label>Title</template>
        <TextInput v-model="data.title" v-koel-focus name="title" required />
      </FormRow>
      <div class="grid grid-cols-[1fr_8rem] gap-2">
        <FormRow>
          <template #label>Album artist</template>
          <TextInput v-model="data.artist" name="artist" required />
        </FormRow>
        <FormRow>
          <template #label>Year</template>
          <TextInput v-model="data.year" max="9999" min="1000" name="year" type="number" />
        </FormRow>
      </div>

      <FormRow v-if="data.tracks.length">
        <template #label>Tracks</template>
        <ul class="flex flex-col gap-1.5 max-h-72 overflow-y-auto">
          <li v-for="track in data.tracks" :key="track.id" class="grid grid-cols-[4rem_1fr] gap-2">
            <TextInput v-model="track.track" :name="`track-${track.id}`" min="1" type="number" />
            <TextInput v-model="track.title" :name="`title-${track.id}`" required />
          </li>
        </ul>
      </FormRow>
      <p v-else class="text-k-fg-70">Loading tracks…</p>
    </main>

    <footer>
      <Btn :disabled="!loaded" type="submit">Save</Btn>
      <Btn variant="ghost" @click.prevent="maybeClose">Cancel</Btn>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { AlbumUpdateData } from '@/stores/albumStore'
import { albumStore } from '@/stores/albumStore'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useForm } from '@/composables/useForm'

import FormRow from '@/components/ui/form/FormRow.vue'
import Btn from '@/components/ui/form/Btn.vue'
import TextInput from '@/components/ui/form/TextInput.vue'

const props = defineProps<{ album: Album }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { handleHttpError } = useErrorHandler('dialog')

const close = () => emit('close')
const loaded = ref(false)
let original = ''

const { data, handleSubmit } = useForm<AlbumUpdateData>({
  initialValues: { title: props.album.name, artist: props.album.artist_name, year: props.album.year, tracks: [] },
  // The form fills in once the tracks load; changes count from there.
  isPristine: (_, current) => JSON.stringify(current) === original,
  onSubmit: async form =>
    await albumStore.update(props.album, {
      ...form,
      year: form.year ? Number(form.year) : null,
      tracks: form.tracks.map(track => ({ ...track, track: track.track ? Number(track.track) : null })),
    }),
  onSuccess: () => {
    toastSuccess('Album updated.')
    close()
  },
})

const maybeClose = async () => {
  if (JSON.stringify(data) === original || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}

onMounted(async () => {
  try {
    const details = await huntingService.albumDetails(props.album)
    Object.assign(data, {
      title: details.title,
      artist: details.artist,
      year: details.year,
      tracks: details.tracks.map(({ id, title, track }) => ({ id, title, track })),
    })
    original = JSON.stringify(data)
    loaded.value = true
  } catch (error: unknown) {
    handleHttpError(error)
    close()
  }
})
</script>
