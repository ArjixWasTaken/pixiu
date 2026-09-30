<template>
  <div class="flex flex-col gap-6" data-testid="users-settings">
    <PendingRegistrations v-if="requests.length" :requests @approve="approve" @deny="deny" />

    <section class="flex flex-col gap-3">
      <header class="flex items-baseline justify-between gap-4">
        <h3 class="m3-title-large">Accounts</h3>
        <p v-if="storage" class="m3-body-medium text-(--schemes-on-surface-variant)">
          {{ pluralize(storage.files, 'file') }} stored · {{ formatBytes(storage.bytes) }}
          <template v-if="storage.shared_files"> · {{ storage.shared_files }} shared</template>
        </p>
      </header>
      <AccountRow
        v-for="account in members"
        :key="account.id"
        :account
        :is-you="String(account.id) === String(currentUser.id)"
        :mail-ready
        @remove="remove(account)"
        @resend-verification="resendVerification(account)"
        @send-reset="sendReset(account)"
        @set-password="setPassword(account, $event)"
        @toggle-role="toggleRole(account)"
        @toggle-status="toggleStatus(account)"
      />
    </section>

    <SettingGroup v-if="adding">
      <template #title>Add an account</template>
      <template #subtitle>
        Everyone has a library of their own. A song two libraries both hold is stored once.
      </template>
      <AddAccountForm @cancel="adding = false" @created="onCreated" />
    </SettingGroup>
    <M3Button v-else class="self-start" icon="person_add" variant="tonal" @click="adding = true">
      Add an account
    </M3Button>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { adminService } from '@/services/adminService'
import { serverSettingsService } from '@/services/serverSettingsService'
import type { ManagedAccount, StoreUsage } from '@/services/adminService'
import { formatBytes, pluralize } from '@/utils/formatters'
import { useAuthorization } from '@/composables/useAuthorization'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { huntingStore } from '@/stores/huntingStore'

import AccountRow from '@/components/screens/settings/admin/AccountRow.vue'
import AddAccountForm from '@/components/screens/settings/admin/AddAccountForm.vue'
import PendingRegistrations from '@/components/screens/settings/admin/PendingRegistrations.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'
import M3Button from '@/components/m3/M3Button.vue'

const { currentUser } = useAuthorization()
const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const accounts = ref<ManagedAccount[]>([])
const storage = ref<StoreUsage | null>(null)
const mailReady = ref(false)
const adding = ref(false)

/** Those asking for an account, and everyone else. */
const requests = computed(() => accounts.value.filter(account => account.status === 'pending'))
const members = computed(() => accounts.value.filter(account => account.status !== 'pending'))

const onCreated = async () => {
  adding.value = false
  await refresh()
}

const refresh = async () => {
  try {
    let settings
    ;[accounts.value, storage.value, settings] = await Promise.all([
      adminService.users(),
      adminService.storage(),
      serverSettingsService.get(),
    ])
    mailReady.value = settings.mail_ready
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

const approve = (account: ManagedAccount) =>
  change(async () => {
    await adminService.approveRegistration(account.id)
    toastSuccess(`${account.username} is approved, and gets an email to confirm their address.`)
    await huntingStore.refresh()
  })

const deny = async (account: ManagedAccount) => {
  if (!(await showConfirmDialog(`Deny ${account.username}’s request? They get a short note, and the request goes.`))) {
    return
  }

  await change(async () => {
    await adminService.denyRegistration(account.id)
    toastSuccess(`${account.username}’s request is denied.`)
    await huntingStore.refresh()
  })
}

const resendVerification = (account: ManagedAccount) =>
  change(async () => {
    await adminService.resendVerification(account.id)
    toastSuccess(`píxiū emailed ${account.username} a new link to confirm their address.`)
  })

const sendReset = (account: ManagedAccount) =>
  change(async () => {
    await adminService.sendPasswordReset(account.id)
    toastSuccess(`píxiū emailed ${account.username} a link to choose a new password. It works for an hour.`)
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
