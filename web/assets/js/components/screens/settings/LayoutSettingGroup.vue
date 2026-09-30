<template>
  <form @submit.prevent="handleSubmit">
    <SettingGroup>
      <template #title>File layout</template>
      <template #subtitle>
        Where píxiū files each song in its music folder. New songs follow it at once; songs already filed move when you
        say so.
      </template>

      <M3TextField
        v-model="data.template"
        class="template"
        label="Layout template"
        name="template"
        required
        supporting-text="Placeholders: {album_artist}, {artist}, {album}, {year}, {genre}, {disc}, {track}, {title}; {track:02} pads with zeros. A part in [brackets] is left out when its value is missing. “/” separates folders, and the file extension is added at the end."
      />

      <div class="preview">
        <p class="m3-label-medium text-(--schemes-on-surface-variant)">
          Example · {{ layout.example.title }}, track {{ layout.example.track ?? 1 }} of {{ layout.example.album }}
        </p>
        <code v-if="preview.path" class="m3-body-medium font-mono break-all text-(--schemes-primary)">{{
          preview.path
        }}</code>
        <p v-else class="m3-body-medium text-(--schemes-error)">{{ preview.error }}</p>
      </div>

      <AlertBox v-if="layout.misplaced" type="warning">
        <div class="flex items-center gap-3">
          <span class="flex-1">
            {{ pluralize(layout.misplaced, 'song') }} are filed under another layout.
            <template v-if="layout.refiling"> Moving them now.</template>
          </span>
          <M3Button v-if="!layout.refiling" variant="text" @click.prevent="refile">Move them</M3Button>
        </div>
      </AlertBox>

      <template #footer>
        <div class="flex gap-2">
          <M3Button :disabled="data.template === layout.template" type="submit">Save</M3Button>
          <M3Button
            v-if="data.template !== layout.default"
            variant="text"
            @click.prevent="data.template = layout.default"
          >
            Reset to default
          </M3Button>
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
import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
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

<style scoped>
.preview {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 16px 0;
  padding: 12px 16px;
  border-radius: 12px;
  background: var(--schemes-surface-container-high);
}
</style>
