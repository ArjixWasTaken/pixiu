<template>
  <SettingGroup>
    <template #title>MusicBrainz</template>
    <template #subtitle>
      píxiū looks every new album up on MusicBrainz, fixing its tags and finding its cover and lyrics.
    </template>

    <p v-if="unlooked">{{ pluralize(unlooked, 'album') }} were never looked up.</p>
    <p v-else class="text-k-fg-70">Every album has been looked up.</p>

    <template v-if="unlooked" #footer>
      <Btn @click.prevent="lookUpAll">Look them up</Btn>
    </template>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { huntingService } from '@/services/huntingService'
import { pluralize } from '@/utils/formatters'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
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
