<template>
  <form
    class="flex flex-col gap-4"
    data-testid="add-account-form"
    @submit.prevent="handleSubmit"
    @keydown.esc="maybeClose"
  >
    <div class="grid md:grid-cols-2 gap-4">
      <M3TextField v-model="data.username" v-koel-focus autocomplete="off" label="Username" name="username" required />
      <M3TextField v-model="data.email" autocomplete="off" label="Email (optional)" name="email" type="email" />
    </div>
    <M3TextField
      v-model="data.password"
      autocomplete="new-password"
      label="Temporary password"
      name="password"
      required
      supporting-text="They choose their own when they first sign in. At least 8 characters."
    />
    <div class="flex flex-wrap items-center justify-between gap-3">
      <M3SegmentedButton v-model="data.role" :segments="roles" />
      <div class="flex gap-2">
        <M3Button type="button" variant="text" @click.prevent="maybeClose">Cancel</M3Button>
        <M3Button :disabled="loading" icon="person_add" type="submit">Make account</M3Button>
      </div>
    </div>
  </form>
</template>

<script lang="ts" setup>
import { adminService } from '@/services/adminService'
import type { Role } from '@/services/accountService'
import { useDialogBox } from '@/composables/useDialogBox'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'

import M3Button from '@/components/m3/M3Button.vue'
import M3SegmentedButton from '@/components/m3/M3SegmentedButton.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const emit = defineEmits<{ (e: 'created'): void; (e: 'cancel'): void }>()

const roles = [
  { id: 'user', label: 'User' },
  { id: 'admin', label: 'Admin' },
]

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()

const { data, loading, handleSubmit, isPristine } = useForm<{
  username: string
  email: string
  password: string
  role: Role
}>({
  initialValues: { username: '', email: '', password: '', role: 'user' },
  validator: ({ username, password }) => username.trim() !== '' && password.length >= 8,
  onSubmit: async account => await adminService.createUser({ ...account }),
  onSuccess: () => {
    toastSuccess(`${data.username} can sign in now, with the temporary password.`)
    data.username = ''
    data.email = ''
    data.password = ''
    data.role = 'user'
    emit('created')
  },
})

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard this account?'))) {
    emit('cancel')
  }
}
</script>
