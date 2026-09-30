<template>
  <div class="flex items-center justify-center min-h-dvh p-6 flex-col gap-5 bg-(--schemes-surface-container)">
    <CredentialsLoginForm @logged-in="$emit('loggedIn')" />
    <SsoLoginOptions @logged-in="$emit('loggedIn')" />
  </div>
</template>

<script lang="ts" setup>
import { onMounted } from 'vue'
import { authService } from '@/services/authService'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { defineAsyncComponent } from '@/utils/helpers'

const CredentialsLoginForm = defineAsyncComponent(() => import('@/components/auth/CredentialsLoginForm.vue'))
const SsoLoginOptions = defineAsyncComponent(() => import('@/components/auth/sso/SsoLoginOptions.vue'))

defineEmits<{ (e: 'loggedIn'): void }>()

const { toastWarning } = useMessageToaster()

onMounted(() => {
  if (authService.hasRedirect()) {
    toastWarning('Please log in first.')
  }
})
</script>
