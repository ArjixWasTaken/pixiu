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

    <M3TextField
      v-model="data.username"
      :label="claiming ? 'Username' : 'Username or email'"
      autocomplete="username"
      autofocus
      name="username"
      required
    />

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

    <p v-if="problem || notice" class="m3-body-medium text-center text-(--schemes-error)" role="alert">
      {{ problem || notice }}
    </p>

    <M3Button class="w-full" data-testid="submit" type="submit">{{ claiming ? 'Create account' : 'Sign in' }}</M3Button>
    <SsoButton v-if="sso && !claiming" :href="authService.ssoStartUrl()" :name="sso.name" />

    <div v-if="!claiming && (passwordReset || registration)" class="flex flex-wrap justify-center gap-2">
      <M3Button v-if="passwordReset" variant="text" @click.prevent="$emit('forgot')">Forgot password?</M3Button>
      <M3Button v-if="registration" variant="text" @click.prevent="$emit('register')">Ask for an account</M3Button>
    </div>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { onBeforeUnmount, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { logger } from '@/utils/logger'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import AuthFormCard from '@/components/auth/AuthFormCard.vue'
import SsoButton from '@/components/auth/SsoButton.vue'

const props = defineProps<{
  /** A fresh píxiū has no admin yet: the form creates the account instead. */
  claiming: boolean
  /** Whether a forgotten password can be reset by email. */
  passwordReset: boolean
  /** Whether anyone may ask for an account. */
  registration: boolean
  /** The single sign-on provider, if people may sign in with it. */
  sso?: { name: string } | null
  /** Why a single sign-on did not work. */
  notice?: string
}>()

const emit = defineEmits<{ (e: 'loggedIn'): void; (e: 'forgot'): void; (e: 'register'): void }>()

const failed = ref(false)
const problem = ref('')
const showPassword = ref(false)

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

    if (!props.claiming) {
      return await authService.login(username, password)
    }

    if (password !== confirm) {
      throw new Error('The passwords do not match.')
    }

    await authService.claim(username, password)
  },
  onSuccess: () => {
    failed.value = false
    data.password = ''
    data.confirm = ''
    emit('loggedIn')
  },
  onError: (error: unknown) => {
    failed.value = true
    logger.error(error)

    // Say what is wrong, in words as well as the shake (a screen reader hears
    // the alert): the server's reason (a wrong password, an account awaiting
    // approval or turned off, a claim's problem).
    const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
    problem.value =
      body?.message ?? (error instanceof Error && error.message ? error.message : 'Signing in didn’t work.')

    clearErrorResetTimer()
    errorResetTimer = window.setTimeout(() => {
      failed.value = false
      errorResetTimer = null
    }, 2000)
  },
})

onBeforeUnmount(clearErrorResetTimer)
</script>
