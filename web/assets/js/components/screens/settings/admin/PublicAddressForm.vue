<template>
  <form class="flex flex-col gap-4" data-testid="public-address-form" @submit.prevent="handleSubmit">
    <M3TextField
      v-model="data.url"
      :supporting-text="
        url ? undefined : 'Not saved yet. This is the address you use now; save it if everyone uses it too.'
      "
      autocomplete="url"
      label="Public address"
      name="public_url"
      type="url"
    />
    <div class="flex justify-end">
      <M3Button type="submit">Save</M3Button>
    </div>
  </form>
</template>

<script lang="ts" setup>
import { watch } from 'vue'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{
  /** The saved address. */
  url: string | null
  /** What to offer while none is saved: the address this browser uses. */
  suggested: string
}>()

const emit = defineEmits<{ (e: 'save', url: string | null): void }>()

const { data, handleSubmit } = useForm<{ url: string }>({
  initialValues: { url: props.url ?? props.suggested },
  useOverlay: false,
  onSubmit: async ({ url }) => emit('save', url.trim() || null),
})

watch(
  () => props.url,
  url => (data.url = url ?? props.suggested),
)
</script>
