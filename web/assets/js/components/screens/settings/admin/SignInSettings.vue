<template>
  <div class="flex flex-col gap-6" data-testid="sign-in-settings">
    <SettingGroup>
      <template #title>Public address</template>
      <template #subtitle>Where people reach píxiū. Links in its emails start with it.</template>
      <PublicAddressForm v-if="settings" :suggested="origin" :url="settings.public_url" @save="savePublicUrl" />
    </SettingGroup>

    <SettingGroup>
      <template #title>Registration</template>
      <RegistrationSwitch
        v-if="settings"
        :mail-ready="settings.mail_ready"
        :open="settings.registration_open"
        @toggle="setRegistration"
      />
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { serverSettingsService } from '@/services/serverSettingsService'
import type { ServerSettings } from '@/services/serverSettingsService'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

import PublicAddressForm from '@/components/screens/settings/admin/PublicAddressForm.vue'
import RegistrationSwitch from '@/components/screens/settings/admin/RegistrationSwitch.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const origin = location.origin
const settings = ref<ServerSettings | null>(null)

const savePublicUrl = async (url: string | null) => {
  try {
    settings.value = await serverSettingsService.setPublicUrl(url)
    toastSuccess(url ? 'Public address saved.' : 'Public address removed.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const setRegistration = async (open: boolean) => {
  try {
    settings.value = await serverSettingsService.setRegistrationOpen(open)
    toastSuccess(open ? 'Anyone may ask for an account now.' : 'Registration is closed.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(async () => {
  try {
    settings.value = await serverSettingsService.get()
  } catch (error: unknown) {
    handleHttpError(error)
  }
})
</script>
