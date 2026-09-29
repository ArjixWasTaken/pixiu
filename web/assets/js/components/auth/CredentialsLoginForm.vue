<template>
  <AuthFormCard :failed data-testid="login-form" @submit="handleSubmit">
    <FormRow v-if="claiming">
      <p class="text-center text-[.95rem] text-k-fg-70">
        Claim this hoard: create the admin account. Subsonic apps sign in with it too.
      </p>
    </FormRow>

    <FormRow>
      <TextInput v-model="data.username" v-koel-focus autocomplete="username" placeholder="Username" required />
    </FormRow>

    <FormRow>
      <PasswordField v-model="data.password" :autocomplete="claiming ? 'new-password' : 'current-password'" placeholder="Password" required />
    </FormRow>

    <FormRow v-if="claiming">
      <PasswordField v-model="data.confirm" autocomplete="new-password" placeholder="Confirm password" required />
    </FormRow>

    <FormRow v-if="problem">
      <p class="text-center text-[.95rem] text-k-danger">{{ problem }}</p>
    </FormRow>

    <FormRow>
      <Btn class="w-full" data-testid="submit" type="submit">{{ claiming ? 'Claim' : 'Log In' }}</Btn>
    </FormRow>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { logger } from '@/utils/logger'
import { useForm } from '@/composables/useForm'

import Btn from '@/components/ui/form/Btn.vue'
import PasswordField from '@/components/ui/form/PasswordField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import FormRow from '@/components/ui/form/FormRow.vue'
import AuthFormCard from '@/components/auth/AuthFormCard.vue'

const emit = defineEmits<{
  (e: 'loggedIn'): void
  (e: 'twoFactorRequired', loginToken: string): void
  (e: 'forgotPassword'): void
}>()

const failed = ref(false)
/** A fresh píxiū has no admin yet: the form claims the hoard instead. */
const claiming = ref(false)
const problem = ref('')

onMounted(async () => {
  try {
    claiming.value = !(await authService.claimed())
  } catch (error: unknown) {
    logger.error(error)
  }
})

let errorResetTimer: number | null = null

const clearErrorResetTimer = () => {
  if (errorResetTimer !== null) {
    window.clearTimeout(errorResetTimer)
    errorResetTimer = null
  }
}

const { data, handleSubmit } = useForm<{ username: string; password: string; confirm: string }>({
  initialValues: { username: '', password: '', confirm: '' },
  onSubmit: async ({ username, password, confirm }) => {
    problem.value = ''

    if (!claiming.value) {
      return await authService.login(username, password)
    }

    if (password !== confirm) {
      throw new Error('The passwords do not match.')
    }

    await authService.claim(username, password)
    return null
  },
  onSuccess: challenge => {
    failed.value = false
    data.password = ''
    data.confirm = ''

    if (challenge) {
      emit('twoFactorRequired', challenge.login_token)
      return
    }

    emit('loggedIn')
  },
  onError: (error: unknown) => {
    failed.value = true
    logger.error(error)

    // A claim explains what is wrong; a failed login just shakes.
    if (claiming.value) {
      const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
      problem.value = body?.message ?? (error instanceof Error ? error.message : 'That did not work.')
    }

    clearErrorResetTimer()
    errorResetTimer = window.setTimeout(() => {
      failed.value = false
      errorResetTimer = null
    }, 2000)
  },
})

onBeforeUnmount(clearErrorResetTimer)
</script>
