<template>
  <div class="flex flex-col gap-6" data-testid="users-settings">
    <SettingGroup>
      <template #title>Add an account</template>
      <template #subtitle>
        Everyone has a library of their own. A song two libraries both hold is stored once.
      </template>
      <AddAccountForm @created="refresh" />
    </SettingGroup>

    <section class="flex flex-col gap-3">
      <header class="flex items-baseline justify-between gap-4">
        <h3 class="m3-title-large">Accounts</h3>
        <p v-if="storage" class="m3-body-medium text-(--schemes-on-surface-variant)">
          {{ pluralize(storage.files, 'file') }} stored · {{ formatBytes(storage.bytes) }}
          <template v-if="storage.shared_files"> · {{ storage.shared_files }} shared</template>
        </p>
      </header>
      <AccountRow
        v-for="account in accounts"
        :key="account.id"
        :account
        :is-you="String(account.id) === String(currentUser.id)"
        @remove="remove(account)"
        @set-password="setPassword(account, $event)"
        @toggle-role="toggleRole(account)"
        @toggle-status="toggleStatus(account)"
      />
    </section>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { adminService } from '@/services/adminService'
import type { ManagedAccount, StoreUsage } from '@/services/adminService'
import { formatBytes, pluralize } from '@/utils/formatters'
import { useAuthorization } from '@/composables/useAuthorization'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'

import AccountRow from '@/components/screens/settings/admin/AccountRow.vue'
import AddAccountForm from '@/components/screens/settings/admin/AddAccountForm.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { currentUser } = useAuthorization()
const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const accounts = ref<ManagedAccount[]>([])
const storage = ref<StoreUsage | null>(null)

const refresh = async () => {
  try {
    ;[accounts.value, storage.value] = await Promise.all([adminService.users(), adminService.storage()])
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

/** Runs a change, then shows the accounts as they are now. */
const change = async (action: () => Promise<unknown>) => {
  try {
    await action()
  } catch (error: unknown) {
    handleHttpError(error)
  }
  await refresh()
}

const toggleRole = (account: ManagedAccount) =>
  change(() => adminService.updateUser(account.id, { role: account.role === 'admin' ? 'user' : 'admin' }))

const toggleStatus = async (account: ManagedAccount) => {
  const turningOff = account.status !== 'disabled'
  if (
    turningOff &&
    !(await showConfirmDialog(
      `Turn ${account.username}’s account off? They are signed out everywhere; their library stays.`,
    ))
  ) {
    return
  }
  await change(() => adminService.updateUser(account.id, { status: turningOff ? 'disabled' : 'active' }))
}

const setPassword = (account: ManagedAccount, password: string) =>
  change(async () => {
    await adminService.setTemporaryPassword(account.id, password)
    toastSuccess(`${account.username} is signed out, and chooses a new password at their next sign-in.`)
  })

const remove = async (account: ManagedAccount) => {
  const freed = account.exclusive_bytes
    ? ` This frees ${formatBytes(account.exclusive_bytes)}; files other libraries play stay.`
    : ''
  if (
    !(await showConfirmDialog(`Delete ${account.username}’s account and library for good?${freed}`, 'Delete account'))
  ) {
    return
  }
  await change(async () => {
    const { freed_bytes } = await adminService.deleteUser(account.id)
    toastSuccess(`${account.username}’s account is gone; ${formatBytes(freed_bytes)} freed.`)
  })
}

onMounted(refresh)
</script>
