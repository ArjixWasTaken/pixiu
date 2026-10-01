<template>
  <AuthFormCard data-testid="forgot-password-form" @submit="handleSubmit">
    <template #title>
      <h1 class="m3-headline-small text-(--schemes-on-surface)">Forgot your password?</h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        {{
          sent
            ? 'If an account has that name or address, píxiū emailed it a link to choose a new password. The link works for an hour.'
            : 'Say who you are, and píxiū emails you a link to choose a new password.'
        }}
      </p>
    </template>

    <template v-if="!sent">
      <M3TextField
        v-model="data.login"
        autocomplete="username"
        autofocus
        label="Username or email"
        name="login"
        required
      />
      <M3Button :disabled="loading" class="w-full" type="submit">Email me a link</M3Button>
    </template>

    <M3Button class="self-center" variant="text" @click.prevent="$emit('back')">Back to sign in</M3Button>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { authService } from '@/services/authService'
import { useForm } from '@/composables/useForm'

import AuthFormCard from '@/components/auth/AuthFormCard.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

defineEmits<{ (e: 'back'): void }>()

const sent = ref(false)

const { data, loading, handleSubmit } = useForm<{ login: string }>({
  initialValues: { login: '' },
  validator: ({ login }) => login.trim() !== '',
  onSubmit: async ({ login }) => await authService.forgot(login.trim()),
  onSuccess: () => (sent.value = true),
})
</script>
