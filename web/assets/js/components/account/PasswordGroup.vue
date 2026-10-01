<template>
  <SettingGroup>
    <template #title>Password</template>
    <template #subtitle> Changing it signs out your other browsers, and apps that use your password. </template>

    <form class="flex flex-col gap-4" data-testid="password-form" @submit.prevent="handleSubmit">
      <M3TextField
        v-if="!temporary"
        v-model="data.current"
        autocomplete="current-password"
        label="Current password"
        name="current_password"
        required
        type="password"
      />
      <M3TextField
        v-model="data.password"
        :supporting-text="`At least ${MIN_LENGTH} characters.`"
        autocomplete="new-password"
        label="New password"
        name="password"
        required
        type="password"
      />
      <M3TextField
        v-model="data.confirmation"
        :error="mismatch"
        :supporting-text="mismatch ? 'The passwords differ.' : undefined"
        autocomplete="new-password"
        label="New password again"
        name="password_confirmation"
        required
        type="password"
      />
      <div class="flex justify-end">
        <M3Button :disabled="loading" type="submit">Change password</M3Button>
      </div>
    </form>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { accountService } from '@/services/accountService'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const props = withDefaults(defineProps<{ temporary?: boolean }>(), { temporary: false })
const emit = defineEmits<{ (e: 'changed'): void }>()

const MIN_LENGTH = 8

const { toastSuccess } = useMessageToaster()

const { data, loading, handleSubmit } = useForm<{ current: string; password: string; confirmation: string }>({
  initialValues: { current: '', password: '', confirmation: '' },
  validator: ({ password, confirmation }) => password.length >= MIN_LENGTH && password === confirmation,
  onSubmit: async ({ current, password }) => await accountService.changePassword(password, current),
  onSuccess: () => {
    data.current = ''
    data.password = ''
    data.confirmation = ''
    toastSuccess(props.temporary ? 'Your password is set.' : 'Password changed.')
    emit('changed')
  },
})

const mismatch = computed(() => data.confirmation !== '' && data.password !== data.confirmation)
</script>
