<template>
  <SettingGroup>
    <template #title>Profile</template>
    <template #subtitle>
      You sign in with your username or email. píxiū sends password resets and alerts to the email.
    </template>

    <form v-if="account" class="flex flex-col gap-4" data-testid="profile-form" @submit.prevent="handleSubmit">
      <M3TextField v-model="data.username" autocomplete="username" label="Username" name="username" required />
      <M3TextField
        v-model="data.email"
        :supporting-text="emailNote"
        autocomplete="email"
        label="Email"
        name="email"
        type="email"
      />
      <div v-if="canResend" class="flex flex-wrap items-center gap-2" data-testid="email-unconfirmed">
        <span class="m3-body-medium text-(--schemes-on-surface-variant)">
          Follow the link píxiū emailed to {{ account.email }} to confirm it.
        </span>
        <M3Button :disabled="resending" variant="text" @click.prevent="resend">Send the link again</M3Button>
      </div>
      <div class="flex justify-end">
        <M3Button :disabled="loading" type="submit">Save</M3Button>
      </div>
    </form>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { accountService } from '@/services/accountService'
import type { Account } from '@/services/accountService'
import { userStore } from '@/stores/userStore'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const account = ref<Account | null>(null)
const resending = ref(false)

/** An address waits to be confirmed, and email can reach it. */
const canResend = computed(() =>
  Boolean(account.value?.email && !account.value.email_verified && account.value.mail_ready),
)

const resend = async () => {
  resending.value = true
  try {
    await accountService.resendVerification()
    toastSuccess(`píxiū emailed a new link to ${account.value?.email}.`)
  } catch (error: unknown) {
    handleHttpError(error)
  } finally {
    resending.value = false
  }
}

const emailNote = computed(() => {
  if (!account.value?.email) {
    return 'Optional, but without one you cannot reset a forgotten password.'
  }

  if (account.value.email_verified) {
    return 'Confirmed.'
  }

  // The line below the field says so, with the link to send again.
  return canResend.value ? undefined : 'Not confirmed yet.'
})

const { data, loading, handleSubmit } = useForm<{ username: string; email: string }>({
  initialValues: { username: '', email: '' },
  onSubmit: async ({ username, email }) => await accountService.updateProfile({ username, email }),
  onSuccess: (updated: Account) => {
    const newEmail = updated.email !== account.value?.email
    account.value = updated
    if (userStore.state.current) {
      userStore.state.current.name = updated.username
      userStore.state.current.email = updated.email ?? ''
    }
    toastSuccess(
      newEmail && updated.email && !updated.email_verified && updated.mail_ready
        ? `Profile saved. Confirm your new email with the link píxiū sent to ${updated.email}.`
        : 'Profile saved.',
    )
  },
})

onMounted(async () => {
  try {
    account.value = await accountService.me()
    data.username = account.value.username
    data.email = account.value.email ?? ''
  } catch (error: unknown) {
    handleHttpError(error)
  }
})
</script>
