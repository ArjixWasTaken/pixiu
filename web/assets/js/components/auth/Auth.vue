<template>
  <div class="flex items-center justify-center min-h-dvh p-6 flex-col gap-5 bg-(--schemes-surface-container)">
    <ForgotPasswordForm v-if="mode === 'forgot'" @back="mode = 'login'" />
    <RegisterForm v-else-if="mode === 'register'" @back="mode = 'login'" />
    <CredentialsLoginForm
      v-else
      :claiming="status?.claimed === false"
      :notice="ssoProblem"
      :password-reset="status?.password_reset ?? false"
      :registration="status?.registration ?? false"
      :sso="status?.sso"
      @forgot="mode = 'forgot'"
      @logged-in="$emit('loggedIn')"
      @register="mode = 'register'"
    />
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import type { AuthStatus } from '@/services/authService'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { defineAsyncComponent } from '@/utils/helpers'
import { logger } from '@/utils/logger'

const CredentialsLoginForm = defineAsyncComponent(() => import('@/components/auth/CredentialsLoginForm.vue'))
const ForgotPasswordForm = defineAsyncComponent(() => import('@/components/auth/ForgotPasswordForm.vue'))
const RegisterForm = defineAsyncComponent(() => import('@/components/auth/RegisterForm.vue'))

defineEmits<{ (e: 'loggedIn'): void }>()

const { toastWarning } = useMessageToaster()

const mode = ref<'login' | 'forgot' | 'register'>('login')
const status = ref<AuthStatus | null>(null)

/** Why a single sign-on sent the browser back here, by its `?sso_error=`. */
const ssoError = new URLSearchParams(location.search).get('sso_error')

const ssoProblem = computed(() => {
  const provider = status.value?.sso?.name ?? 'single sign-on'

  switch (ssoError) {
    case null:
      return ''
    case 'unlinked':
      return `This ${provider} account is not linked to a píxiū account. Sign in with your password, then link it under Settings → Account.`
    case 'pending':
      return 'An admin has yet to approve your account; you will get an email when they do.'
    case 'unverified':
      return 'Confirm your email address first: follow the link píxiū sent you.'
    case 'disabled':
      return 'Your account is turned off.'
    case 'expired':
      return 'That sign-in took too long, or started in another browser. Try again.'
    case 'denied':
      return 'The sign-in was cancelled.'
    case 'busy':
      return 'Too many tries; wait a while and try again.'
    case 'unavailable':
      return 'Single sign-on is not set up.'
    default:
      return `Signing in with ${provider} did not work. Try again, or sign in with your password.`
  }
})

onMounted(async () => {
  if (authService.hasRedirect()) {
    toastWarning('Sign in first.')
  }

  if (ssoError) {
    // Said once: a reload should not say it again.
    history.replaceState(history.state, '', location.pathname)
  }

  try {
    status.value = await authService.status()
  } catch (error: unknown) {
    logger.error(error)
  }
})
</script>
