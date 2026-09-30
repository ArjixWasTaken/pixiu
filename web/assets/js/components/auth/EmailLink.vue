<template>
  <div class="flex items-center justify-center min-h-dvh p-6 bg-(--schemes-surface-container)" data-testid="email-link">
    <ResetPasswordForm v-if="screen === 'ResetPassword'" :token @done="openPlayer" />
    <VerifyEmailCard v-else :state @continue="openPlayer" />
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { useRouter } from '@/composables/useRouter'
import { logger } from '@/utils/logger'

import ResetPasswordForm from '@/components/auth/ResetPasswordForm.vue'
import VerifyEmailCard from '@/components/auth/VerifyEmailCard.vue'
import type { VerificationState } from '@/components/auth/VerifyEmailCard.vue'

const { getCurrentScreen, getRouteParam } = useRouter()

const screen = getCurrentScreen()
const token = getRouteParam('token') ?? ''
const state = ref<VerificationState>('confirming')

/** Loads the player afresh: signed in by now, or at the sign-in screen. */
const openPlayer = () => location.assign('/')

onMounted(async () => {
  if (screen !== 'VerifyEmail') {
    return
  }

  try {
    state.value = (await authService.verifyEmail(token)) ? 'opened' : 'confirmed'
  } catch (error: unknown) {
    logger.error(error)
    state.value = isHttpError(error) && getHttpErrorBody(error)?.code === 'expired' ? 'expired' : 'failed'
  }
})
</script>
