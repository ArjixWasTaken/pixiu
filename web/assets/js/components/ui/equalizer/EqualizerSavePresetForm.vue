<template>
  <form class="flex gap-2 items-center w-full" @submit.prevent="handleSubmit" @keydown.esc="maybeCancel">
    <M3TextField v-model="data.name" v-koel-focus class="flex-1" label="Preset name" name="preset-name" required />
    <M3Button type="button" variant="text" @click.prevent="maybeCancel">Cancel</M3Button>
    <M3Button type="submit">Save</M3Button>
  </form>
</template>

<script lang="ts" setup>
import { useDialogBox } from '@/composables/useDialogBox'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const emit = defineEmits<{
  (e: 'submit', name: string): void
  (e: 'cancel'): void
}>()

const { showConfirmDialog } = useDialogBox()

const { data, isPristine, handleSubmit } = useForm<{ name: string }>({
  initialValues: { name: '' },
  validator: ({ name }) => name.trim().length > 0,
  onSubmit: async ({ name }) => emit('submit', name.trim()),
  useOverlay: false,
})

const maybeCancel = async () => {
  if (isPristine() || (await showConfirmDialog('Discard the preset’s name?', { action: 'Discard' }))) {
    emit('cancel')
  }
}
</script>
