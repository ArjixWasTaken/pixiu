<template>
  <form @submit.prevent="handleSubmit">
    <SettingGroup>
      <template #title>File layout</template>
      <template #subtitle>
        Where píxiū files each song in the treasure directory. New songs follow it at once; songs already filed move
        when you say so.
      </template>

      <FormRow>
        <template #help>
          Placeholders: {album_artist}, {artist}, {album}, {year}, {genre}, {disc}, {track}, {title}; {track:02} pads
          with zeros. A part in [brackets] is left out when its value is missing. “/” separates folders, and the file
          extension is added at the end.
        </template>
        <TextInput v-model="data.template" class="font-mono" name="template" required />
      </FormRow>

      <div class="preview">
        <p class="text-xs uppercase tracking-widest text-k-fg-50">
          Example · {{ layout.example.title }}, track {{ layout.example.track ?? 1 }} of {{ layout.example.album }}
        </p>
        <code v-if="preview.path" class="font-mono text-sm break-all text-k-highlight">{{ preview.path }}</code>
        <p v-else class="text-sm text-k-danger">{{ preview.error }}</p>
      </div>

      <AlertBox v-if="layout.misplaced" type="info">
        {{ pluralize(layout.misplaced, 'song') }} are filed under another layout.
        <template v-if="layout.refiling"> Moving them now.</template>
        <a v-else role="button" @click.prevent="refile">Move them</a>
      </AlertBox>

      <template #footer>
        <div class="flex gap-2">
          <Btn :disabled="data.template === layout.template" type="submit">Save</Btn>
          <Btn v-if="data.template !== layout.default" variant="ghost" @click.prevent="data.template = layout.default">
            Reset to default
          </Btn>
        </div>
      </template>
    </SettingGroup>
  </form>
</template>

<script lang="ts" setup>
import { onBeforeUnmount, reactive, watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Settings } from '@/services/huntingService'
import { isHttpError, getHttpErrorBody } from '@/services/http'
import { pluralize } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import AlertBox from '@/components/ui/AlertBox.vue'
import Btn from '@/components/ui/form/Btn.vue'
import FormRow from '@/components/ui/form/FormRow.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const props = defineProps<{ layout: Settings['layout'] }>()
const emit = defineEmits<{ (e: 'changed'): void }>()

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const preview = reactive({ path: '', error: '' })

const { data, handleSubmit } = useForm<{ template: string }>({
  initialValues: { template: props.layout.template },
  onSubmit: async ({ template }) => await huntingService.saveLayout(template),
  onSuccess: () => {
    toastSuccess('Saved. New songs follow the new layout.')
    emit('changed')
  },
})

let previewTimer: number | undefined

// Shows where the draft would file the example, as the admin types.
const updatePreview = async (template: string) => {
  try {
    preview.path = (await huntingService.previewLayout(template)).path
    preview.error = ''
  } catch (error: unknown) {
    preview.path = ''
    preview.error = isHttpError(error) ? (getHttpErrorBody(error)?.message ?? 'Invalid layout') : 'Invalid layout'
  }
}

watch(
  () => data.template,
  template => {
    window.clearTimeout(previewTimer)
    previewTimer = window.setTimeout(() => updatePreview(template), 250)
  },
  { immediate: true },
)

onBeforeUnmount(() => window.clearTimeout(previewTimer))

const refile = async () => {
  try {
    await huntingService.refile()
    toastSuccess('Moving the files. Jobs shows how it goes.')
    emit('changed')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.preview {
  @apply flex flex-col gap-1.5 rounded-lg border border-k-fg-10 bg-k-fg-3 px-4 py-3 mb-4;
}
</style>
