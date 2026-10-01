<template>
  <div
    class="flex items-center justify-center min-h-dvh p-6 bg-(--schemes-surface-container)"
    data-testid="sso-complete"
  >
    <AuthFormCard @submit="backToSignIn">
      <template #title>
        <h1 class="m3-headline-small text-(--schemes-on-surface)">
          {{ failed ? 'That sign-in expired' : 'Signing you in…' }}
        </h1>
        <p v-if="failed" class="m3-body-medium text-(--schemes-on-surface-variant)">
          It has to finish within a minute. Try again.
        </p>
      </template>

      <M3Button v-if="failed" class="w-full" type="submit">Back to sign in</M3Button>
      <M3ProgressIndicator v-else class="self-center" />
    </AuthFormCard>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { useRouter } from '@/composables/useRouter'
import { logger } from '@/utils/logger'

import AuthFormCard from '@/components/auth/AuthFormCard.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

const { getRouteParam } = useRouter()

const failed = ref(false)

const backToSignIn = () => location.assign('/')

onMounted(async () => {
  try {
    await authService.exchangeSsoCode(getRouteParam('code') ?? '')
    // Signed in: the player loads afresh, as after any sign-in.
    location.assign('/')
  } catch (error: unknown) {
    // An old or used code is no fault: the screen says so.
    if (!(isHttpError(error) && getHttpErrorBody(error)?.code === 'expired')) {
      logger.error(error)
    }
    failed.value = true
  }
})
</script>
