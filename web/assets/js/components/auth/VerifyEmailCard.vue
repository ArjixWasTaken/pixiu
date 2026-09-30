<template>
  <AuthFormCard data-testid="verify-email" @submit="$emit('continue')">
    <template #title>
      <h1 class="m3-headline-small text-(--schemes-on-surface)">{{ copy.title }}</h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">{{ copy.text }}</p>
    </template>

    <M3ProgressIndicator v-if="state === 'confirming'" class="self-center" />
    <M3Button v-else class="w-full" type="submit">Open píxiū</M3Button>
  </AuthFormCard>
</template>

<script lang="ts" setup>
import { computed } from 'vue'

import AuthFormCard from '@/components/auth/AuthFormCard.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

export type VerificationState = 'confirming' | 'confirmed' | 'expired' | 'failed'

const props = defineProps<{ state: VerificationState }>()
defineEmits<{ (e: 'continue'): void }>()

const copy = computed(
  () =>
    ({
      confirming: { title: 'Confirming your email…', text: 'This takes a moment.' },
      confirmed: {
        title: 'Your email is confirmed',
        text: 'píxiū can send you password resets and alerts now.',
      },
      expired: {
        title: 'This link no longer works',
        text: 'It expired, or was used already. Sign in and ask for a new one under Settings → Account.',
      },
      failed: { title: 'That did not work', text: 'píxiū could not confirm your email. Try the link again later.' },
    })[props.state],
)
</script>
