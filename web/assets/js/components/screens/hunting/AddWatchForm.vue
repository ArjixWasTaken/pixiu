<template>
  <form class="flex flex-col gap-3 max-w-[720px]" @submit.prevent="handleSubmit">
    <div class="flex gap-2">
      <TextInput
        v-model="data.target"
        class="flex-1"
        name="target"
        placeholder="A playlist or artist link from music.youtube.com"
        required
      />
      <Btn type="submit">Watch</Btn>
    </div>

    <div v-if="isArtist" class="flex flex-wrap gap-6">
      <label class="flex items-center gap-2">
        <CheckBox v-model="data.onlyNew" name="only_new" />
        Only releases from now on
      </label>
      <label class="flex items-center gap-2">
        <CheckBox v-model="data.singles" name="singles" />
        Singles and EPs too
      </label>
    </div>

    <p class="text-k-fg-70 text-sm">
      Or
      <a role="button" @click.prevent="watchLiked">watch your liked music</a>
      (needs a YouTube Music login).
    </p>
  </form>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { huntingService } from '@/services/huntingService'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import CheckBox from '@/components/ui/form/CheckBox.vue'
import TextInput from '@/components/ui/form/TextInput.vue'

const emit = defineEmits<{ (e: 'added'): void }>()

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const { data, handleSubmit } = useForm<{ target: string; onlyNew: boolean; singles: boolean }>({
  initialValues: { target: '', onlyNew: true, singles: false },
  validator: ({ target }) => target.trim() !== '',
  onSubmit: async ({ target, onlyNew, singles }) => await huntingService.addWatch(target.trim(), onlyNew, singles),
  onSuccess: () => {
    toastSuccess('Watching. The first sync is on its way.')
    data.target = ''
    emit('added')
  },
})

/** Artist links (channels) get a choice of what to follow. */
const isArtist = computed(() => /\/channel\/|browse\/UC/.test(data.target))

const watchLiked = async () => {
  try {
    await huntingService.addWatch('liked')
    toastSuccess('Watching your liked music.')
    emit('added')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}
</script>
