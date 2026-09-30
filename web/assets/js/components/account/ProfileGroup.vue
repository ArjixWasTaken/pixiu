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

const emailNote = computed(() => {
  if (!account.value?.email) {
    return 'Optional, but without one you cannot reset a forgotten password.'
  }

  return account.value.email_verified ? 'Confirmed.' : 'Not confirmed yet.'
})

const { data, loading, handleSubmit } = useForm<{ username: string; email: string }>({
  initialValues: { username: '', email: '' },
  onSubmit: async ({ username, email }) => await accountService.updateProfile({ username, email }),
  onSuccess: (updated: Account) => {
    account.value = updated
    if (userStore.state.current) {
      userStore.state.current.name = updated.username
      userStore.state.current.email = updated.email ?? ''
    }
    toastSuccess('Profile saved.')
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
