<template>
  <form class="flex flex-col gap-3 max-w-[720px]" @submit.prevent="handleSubmit">
    <div class="flex gap-3 items-start flex-wrap">
      <M3TextField
        v-model="data.target"
        class="flex-[1_1_240px] min-w-0"
        label="Playlist or artist link"
        name="target"
        placeholder="https://music.youtube.com/…"
        required
      />
      <M3Button class="mt-2" type="submit">Watch</M3Button>
    </div>

    <div v-if="isArtist" class="flex flex-wrap gap-6">
      <label class="flex items-center gap-2 m3-body-large">
        <M3Switch v-model="data.onlyNew" name="only_new" />
        Only releases from now on
      </label>
      <label class="flex items-center gap-2 m3-body-large">
        <M3Switch v-model="data.singles" name="singles" />
        Singles and EPs too
      </label>
    </div>

    <p v-if="!likedWatched" class="m3-body-small px-4 text-(--schemes-on-surface-variant)">
      Or
      <a class="text-(--schemes-primary)" role="button" @click.prevent="watchLiked">watch your liked music</a>
      (you sign in to YouTube Music for it).
    </p>
  </form>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { huntingService } from '@/services/huntingService'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Switch from '@/components/m3/M3Switch.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

/** Whether liked music is watched already: then it isn't offered. */
defineProps<{ likedWatched?: boolean }>()

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
