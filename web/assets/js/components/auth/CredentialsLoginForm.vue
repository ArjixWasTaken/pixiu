<template>
  <AuthFormCard :failed data-testid="login-form" @submit="handleSubmit">
    <template #title>
      <h1 class="m3-headline-small text-(--schemes-on-surface)">
        {{ claiming ? 'Welcome to píxiū' : 'Sign in to píxiū' }}
      </h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        {{
          claiming
            ? 'Create the admin account; Subsonic apps sign in with it too.'
            : 'Use the same account in your Subsonic apps.'
        }}
      </p>
    </template>

    <M3TextField v-model="data.username" autocomplete="username" autofocus label="Username" name="username" required />

    <M3TextField
      v-model="data.password"
      :autocomplete="claiming ? 'new-password' : 'current-password'"
      :type="showPassword ? 'text' : 'password'"
      label="Password"
      name="password"
      required
    >
      <template #trailing>
        <M3IconButton
          :icon="showPassword ? 'visibility_off' : 'visibility'"
          :label="showPassword ? 'Hide password' : 'Show password'"
          @click="showPassword = !showPassword"
        />
      </template>
    </M3TextField>

    <M3TextField
      v-if="claiming"
      v-model="data.confirm"
      autocomplete="new-password"
      label="Confirm password"
      name="confirm"
      required
      type="password"
    />

    <p v-if="problem" class="m3-body-medium text-center text-(--schemes-error)">{{ problem }}</p>

    <M3Button class="w-full" data-testid="submit" type="submit">{{ claiming ? 'Create account' : 'Log in' }}</M3Button>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { logger } from '@/utils/logger'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import AuthFormCard from '@/components/auth/AuthFormCard.vue'

const emit = defineEmits<{
  (e: 'loggedIn'): void
  (e: 'twoFactorRequired', loginToken: string): void
  (e: 'forgotPassword'): void
}>()

const failed = ref(false)
/** A fresh píxiū has no admin yet: the form creates the account instead. */
const claiming = ref(false)
const problem = ref('')
const showPassword = ref(false)

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
