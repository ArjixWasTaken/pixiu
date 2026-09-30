<template>
  <SettingGroup>
    <template #title>MusicBrainz</template>
    <template #subtitle>
      píxiū looks every new album up on MusicBrainz, fixing its tags and finding its cover and lyrics.
    </template>

    <p v-if="unlooked" class="m3-body-large">{{ pluralize(unlooked, 'album') }} were never looked up.</p>
    <p v-else class="m3-body-large text-(--schemes-on-surface-variant)">Every album has been looked up.</p>

    <template v-if="unlooked" #footer>
      <M3Button variant="tonal" @click.prevent="lookUpAll">Look them up</M3Button>
    </template>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { huntingService } from '@/services/huntingService'
import { pluralize } from '@/utils/formatters'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

defineProps<{ unlooked: number }>()
const emit = defineEmits<{ (e: 'changed'): void }>()

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const lookUpAll = async () => {
  try {
    const { queued } = await huntingService.lookUpAll()
    toastSuccess(`Looking up ${pluralize(queued, 'album')}. Jobs shows how it goes.`)
    emit('changed')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}
</script>
