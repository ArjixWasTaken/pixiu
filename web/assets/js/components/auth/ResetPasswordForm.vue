<template>
  <AuthFormCard :failed="Boolean(problem)" data-testid="reset-password-form" @submit="handleSubmit">
    <template #title>
      <h1 class="m3-headline-small text-(--schemes-on-surface)">Choose a new password</h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        Your browsers and apps signed in with the old one sign out.
      </p>
    </template>

    <template v-if="!expired">
      <M3TextField
        v-model="data.password"
        :supporting-text="`At least ${MIN_LENGTH} characters.`"
        autocomplete="new-password"
        autofocus
        label="New password"
        name="password"
        required
        type="password"
      />
      <M3TextField
        v-model="data.confirmation"
        :error="mismatch"
        :supporting-text="mismatch ? 'The passwords differ.' : undefined"
        autocomplete="new-password"
        label="New password again"
        name="password_confirmation"
        required
        type="password"
      />
    </template>

    <p v-if="problem" class="m3-body-medium text-center text-(--schemes-error)">{{ problem }}</p>

    <M3Button v-if="expired" class="w-full" @click.prevent="$emit('done')">Back to sign in</M3Button>
    <M3Button v-else :disabled="loading" class="w-full" type="submit">Set password</M3Button>
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

const props = defineProps<{ token: string }>()
const emit = defineEmits<{ (e: 'done'): void }>()

const MIN_LENGTH = 8

const problem = ref('')
const expired = ref(false)

const { data, loading, handleSubmit } = useForm<{ password: string; confirmation: string }>({
  initialValues: { password: '', confirmation: '' },
  useOverlay: false,
  validator: ({ password, confirmation }) => password.length >= MIN_LENGTH && password === confirmation,
  onSubmit: async ({ password }) => {
    problem.value = ''
    await authService.resetPassword(props.token, password)
  },
  onSuccess: () => emit('done'),
  onError: (error: unknown) => {
    const body = isHttpError(error) ? getHttpErrorBody(error) : undefined
    expired.value = body?.code === 'expired'
    problem.value = expired.value
      ? 'This link expired, or was used already. Ask for a new one from the sign-in screen.'
      : (body?.message ?? 'That did not work; try again.')
  },
})

const mismatch = computed(() => data.confirmation !== '' && data.password !== data.confirmation)
</script>
