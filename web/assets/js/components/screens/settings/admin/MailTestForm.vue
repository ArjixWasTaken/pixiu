<template>
  <form class="flex flex-wrap items-start gap-3" data-testid="mail-test-form" @submit.prevent="handleSubmit">
    <M3TextField
      v-model="data.to"
      autocomplete="email"
      class="flex-1 min-w-60"
      label="Send a test email to"
      name="to"
      required
      type="email"
    />
    <M3Button :disabled="busy" class="mt-2" icon="send" type="submit">Send</M3Button>
  </form>
</template>

<script lang="ts" setup>
import { watch } from 'vue'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{ defaultTo: string; busy: boolean }>()
const emit = defineEmits<{ (e: 'send', to: string): void }>()

const { data, handleSubmit } = useForm<{ to: string }>({
  initialValues: { to: props.defaultTo },
  useOverlay: false,
  onSubmit: async ({ to }) => emit('send', to.trim()),
})

watch(
  () => props.defaultTo,
  to => (data.to ||= to),
)
</script>
