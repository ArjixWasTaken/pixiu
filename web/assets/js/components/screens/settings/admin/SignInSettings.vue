<template>
  <div class="flex flex-col gap-6" data-testid="sign-in-settings">
    <SettingGroup>
      <template #title>Public address</template>
      <template #subtitle>Where people reach píxiū. Links in its emails start with it.</template>
      <PublicAddressForm v-if="loaded" :suggested="origin" :url="publicUrl" @save="save" />
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { serverSettingsService } from '@/services/serverSettingsService'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

import PublicAddressForm from '@/components/screens/settings/admin/PublicAddressForm.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const origin = location.origin
const loaded = ref(false)
const publicUrl = ref<string | null>(null)

const save = async (url: string | null) => {
  try {
    publicUrl.value = (await serverSettingsService.setPublicUrl(url)).public_url
    toastSuccess(publicUrl.value ? 'Public address saved.' : 'Public address removed.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(async () => {
  try {
    publicUrl.value = (await serverSettingsService.get()).public_url
    loaded.value = true
  } catch (error: unknown) {
    handleHttpError(error)
  }
})
</script>
