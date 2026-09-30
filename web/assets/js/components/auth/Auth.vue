<template>
  <div class="flex items-center justify-center min-h-dvh p-6 flex-col gap-5 bg-(--schemes-surface-container)">
    <ForgotPasswordForm v-if="mode === 'forgot'" @back="mode = 'login'" />
    <template v-else>
      <CredentialsLoginForm
        :claiming="status?.claimed === false"
        :password-reset="status?.password_reset ?? false"
        @forgot="mode = 'forgot'"
        @logged-in="$emit('loggedIn')"
      />
      <SsoLoginOptions @logged-in="$emit('loggedIn')" />
    </template>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import type { AuthStatus } from '@/services/authService'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { defineAsyncComponent } from '@/utils/helpers'
import { logger } from '@/utils/logger'

const CredentialsLoginForm = defineAsyncComponent(() => import('@/components/auth/CredentialsLoginForm.vue'))
const ForgotPasswordForm = defineAsyncComponent(() => import('@/components/auth/ForgotPasswordForm.vue'))
const SsoLoginOptions = defineAsyncComponent(() => import('@/components/auth/sso/SsoLoginOptions.vue'))

defineEmits<{ (e: 'loggedIn'): void }>()

const { toastWarning } = useMessageToaster()

const mode = ref<'login' | 'forgot'>('login')
const status = ref<AuthStatus | null>(null)

onMounted(async () => {
  if (authService.hasRedirect()) {
    toastWarning('Please log in first.')
  }

  try {
    status.value = await authService.status()
  } catch (error: unknown) {
    logger.error(error)
  }
})
</script>
