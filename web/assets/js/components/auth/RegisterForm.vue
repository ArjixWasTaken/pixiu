<template>
  <AuthFormCard :failed="Boolean(problem)" data-testid="register-form" @submit="handleSubmit">
    <template #title>
      <h1 class="m3-headline-small text-(--schemes-on-surface)">
        {{ sent ? 'Thanks for asking' : 'Ask for an account' }}
      </h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        {{
          sent
            ? `An admin reviews your request, and píxiū emails ${data.email} how it went. If approved, the email has a link to confirm your address; then you can sign in.`
            : 'An admin reviews each request. Your library starts empty, and is yours alone.'
        }}
      </p>
    </template>

    <template v-if="!sent">
      <M3TextField
        v-model="data.username"
        autocomplete="username"
        autofocus
        label="Username"
        name="username"
        required
      />
      <M3TextField v-model="data.email" autocomplete="email" label="Email" name="email" required type="email" />
      <M3TextField
        v-model="data.password"
        :supporting-text="`At least ${MIN_LENGTH} characters.`"
        autocomplete="new-password"
        label="Password"
        name="password"
        required
        type="password"
      />
      <M3TextField
        v-model="data.confirmation"
        :error="mismatch"
        :supporting-text="mismatch ? 'The passwords differ.' : undefined"
        autocomplete="new-password"
        label="Password again"
        name="password_confirmation"
        required
        type="password"
      />

      <p v-if="problem" class="m3-body-medium text-center text-(--schemes-error)">{{ problem }}</p>

      <M3Button :disabled="loading" class="w-full" type="submit">Ask for an account</M3Button>
    </template>

    <M3Button class="self-center" variant="text" @click.prevent="$emit('back')">Back to sign in</M3Button>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { authService } from '@/services/authService'
import { getHttpErrorBody, isHttpError } from '@/services/http'
import { useForm } from '@/composables/useForm'

import AuthFormCard from '@/components/auth/AuthFormCard.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

defineEmits<{ (e: 'back'): void }>()

const MIN_LENGTH = 8

const sent = ref(false)
const problem = ref('')

const { data, loading, handleSubmit } = useForm<{
  username: string
  email: string
  password: string
  confirmation: string
}>({
  initialValues: { username: '', email: '', password: '', confirmation: '' },
  useOverlay: false,
  validator: ({ username, email, password, confirmation }) =>
    username.trim() !== '' && email.trim() !== '' && password.length >= MIN_LENGTH && password === confirmation,
  onSubmit: async ({ username, email, password }) => {
    problem.value = ''
    await authService.register({ username: username.trim(), email: email.trim(), password })
  },
  onSuccess: () => {
    data.password = ''
    data.confirmation = ''
    sent.value = true
  },
  onError: (error: unknown) => {
    const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
    problem.value = body?.message ?? 'That did not work; try again.'
  },
})

const mismatch = computed(() => data.confirmation !== '' && data.password !== data.confirmation)
</script>
