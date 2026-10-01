<template>
  <div class="flex flex-col gap-6" data-testid="sign-in-settings">
    <SettingGroup>
      <template #title>Public address</template>
      <template #subtitle>Where people reach píxiū. Links in its emails start with it.</template>
      <PublicAddressForm v-if="settings" :suggested="origin" :url="settings.public_url" @save="savePublicUrl" />
    </SettingGroup>

    <SettingGroup>
      <template #title>Single sign-on</template>
      <template #subtitle>
        An OpenID Connect provider (Authelia, say). People sign in with it once they link it under Settings → Account;
        it never makes accounts.
      </template>
      <SingleSignOnForm
        v-if="settings"
        :oidc="settings.oidc"
        :redirect-uri="settings.redirect_uri"
        :testing
        @copied="toastSuccess('Copied.')"
        @remove="removeSingleSignOn"
        @save="saveSingleSignOn"
        @test="testSingleSignOn"
      />
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
import type { ServerSettings, SingleSignOnForm as Form } from '@/services/serverSettingsService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

import PublicAddressForm from '@/components/screens/settings/admin/PublicAddressForm.vue'
import RegistrationSwitch from '@/components/screens/settings/admin/RegistrationSwitch.vue'
import SingleSignOnForm from '@/components/screens/settings/admin/SingleSignOnForm.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')
const { showConfirmDialog, showErrorDialog, showSuccessDialog } = useDialogBox()

const origin = location.origin
const settings = ref<ServerSettings | null>(null)
const testing = ref(false)

const saveSingleSignOn = async (form: Form) => {
  try {
    settings.value = await serverSettingsService.setSingleSignOn(form)
    toastSuccess(`Saved. The sign-in screen offers ${form.name} now.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const removeSingleSignOn = async () => {
  if (!(await showConfirmDialog('Remove single sign-on? People sign in with their passwords again.'))) {
    return
  }

  try {
    settings.value = await serverSettingsService.removeSingleSignOn()
    toastSuccess('Single sign-on removed.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const testSingleSignOn = async (form: Form) => {
  testing.value = true
  try {
    const report = await serverSettingsService.testSingleSignOn(form)
    const keys = report.keys === 1 ? 'one signing key' : `${report.keys} signing keys`
    if (report.pkce_s256) {
      showSuccessDialog(`${form.name} answers, with ${keys}, and takes PKCE (S256).`, 'It works')
    } else {
      showErrorDialog(
        `${form.name} answers, with ${keys}, but does not say it takes PKCE (S256), which píxiū uses. Turn it on for the client.`,
        'Almost',
      )
    }
  } catch (error: unknown) {
    const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
    if (body?.message) {
      showErrorDialog(body.message, 'The provider did not answer right')
    } else {
      handleHttpError(error)
    }
  } finally {
    testing.value = false
  }
}

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
