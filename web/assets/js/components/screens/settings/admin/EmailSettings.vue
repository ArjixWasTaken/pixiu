<template>
  <div class="flex flex-col gap-6" data-testid="email-settings">
    <SettingGroup>
      <template #title>Mail server</template>
      <template #subtitle> píxiū sends password resets, email confirmations and alerts through it. </template>

      <p v-if="settings && !settings.public_url" class="note m3-body-medium mb-4" data-testid="public-address-missing">
        Emails link back to píxiū: save its public address under Sign-in too.
      </p>
      <MailServerForm v-if="settings" :server="settings.smtp" @remove="remove" @save="save" />
    </SettingGroup>

    <SettingGroup v-if="settings?.smtp">
      <template #title>Test</template>
      <template #subtitle>Sends an email now, and shows what the mail server answers if it fails.</template>
      <MailTestForm :busy="testing" :default-to="currentUser.email ?? ''" @send="test" />
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { serverSettingsService } from '@/services/serverSettingsService'
import type { MailServerForm as Form, ServerSettings } from '@/services/serverSettingsService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { useAuthorization } from '@/composables/useAuthorization'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

import MailServerForm from '@/components/screens/settings/admin/MailServerForm.vue'
import MailTestForm from '@/components/screens/settings/admin/MailTestForm.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { currentUser } = useAuthorization()
const { showConfirmDialog, showErrorDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const settings = ref<ServerSettings | null>(null)
const testing = ref(false)

const save = async (form: Form) => {
  try {
    settings.value = await serverSettingsService.setMailServer(form)
    toastSuccess('Mail server saved. Send a test email to be sure it works.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const remove = async () => {
  if (
    !(await showConfirmDialog('Remove the mail server? píxiū stops sending email, password resets included.', {
      action: 'Remove',
    }))
  ) {
    return
  }

  try {
    settings.value = await serverSettingsService.removeMailServer()
    toastSuccess('Mail server removed.')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const test = async (to: string) => {
  testing.value = true
  try {
    await serverSettingsService.testMailServer(to)
    toastSuccess(`Sent. Look for it at ${to}.`)
  } catch (error: unknown) {
    const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
    if (body?.code === 'smtp') {
      showErrorDialog(body.message ?? '', 'The test email did not go out')
    } else {
      handleHttpError(error)
    }
  } finally {
    testing.value = false
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

<style scoped>
.note {
  padding: 12px 16px;
  border-radius: 12px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);
}
</style>
