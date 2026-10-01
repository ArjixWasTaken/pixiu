<template>
  <form class="md:w-[480px] min-w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>Edit artist</h1>
    </header>

    <main class="flex flex-col gap-4 pt-2">
      <M3TextField v-model="data.name" v-koel-focus label="Name" name="name" required />
      <ArtworkField v-model="data.image">Pick or paste an image (optional)</ArtworkField>
    </main>

    <footer>
      <M3Button variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script setup lang="ts">
import { pick } from 'lodash-es'
import { toRaw } from 'vue'

import type { ArtistUpdateData } from '@/stores/artistStore'
import { artistStore } from '@/stores/artistStore'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useDialogBox } from '@/composables/useDialogBox'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import ArtworkField from '@/components/ui/form/ArtworkField.vue'

const props = defineProps<{ artist: Artist }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { artist } = props

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const close = () => emit('close')

const { data, isPristine, handleSubmit } = useForm<ArtistUpdateData>({
  initialValues: { ...pick(artist, 'name', 'image') },
  onSubmit: async data => {
    const formData = structuredClone(toRaw(data))

    if (formData.image === artist.image) {
      // If the image is the same, don't send it (the image URL) to the server.
      delete formData.image
    }

    await artistStore.update(artist, formData)
  },
  onSuccess: () => {
    toastSuccess('Artist updated.')
    close()
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>
