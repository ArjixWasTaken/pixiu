<template>
  <LoginBrowser v-if="signingIn" @cancel="signingIn = false" @done="onSignedIn" />

  <div v-else-if="sources" class="flex flex-col gap-8 md:w-2/3">
    <SettingGroup>
      <template #title>YouTube Music account</template>
      <template #subtitle>
        A login lets píxiū watch your liked music and download at full quality. Everything else works without one.
      </template>

      <AlertBox :type="look.alert">
        <strong>{{ look.title }}</strong>
        <p class="text-sm">{{ look.text }}</p>
      </AlertBox>

      <dl v-if="sources.health.state !== 'none'" class="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 text-sm">
        <dt class="text-k-fg-70">Connected</dt>
        <dd>{{ when(sources.health.connected_at) }}</dd>
        <dt class="text-k-fg-70">Last checked</dt>
        <dd>{{ when(sources.health.last_verified) }}</dd>
        <dt class="text-k-fg-70">Last refreshed</dt>
        <dd>{{ when(sources.health.last_refreshed) }}</dd>
        <template v-if="sources.health.expired_at">
          <dt class="text-k-fg-70">Expired</dt>
          <dd>{{ when(sources.health.expired_at) }}</dd>
        </template>
        <template v-if="sources.health.last_error">
          <dt class="text-k-fg-70">Last error</dt>
          <dd class="break-words">{{ sources.health.last_error }}</dd>
        </template>
      </dl>

      <template #footer>
        <div class="flex flex-wrap gap-2">
          <Btn @click.prevent="signIn">{{ sources.health.state === 'none' ? 'Connect' : 'Log in again' }}</Btn>
          <template v-if="sources.health.state !== 'none'">
            <Btn variant="ghost" @click.prevent="check">Check now</Btn>
            <Btn variant="ghost" @click.prevent="refreshCookies">Refresh</Btn>
            <Btn variant="destructive" @click.prevent="disconnect">Disconnect</Btn>
          </template>
        </div>
      </template>
    </SettingGroup>

    <SettingGroup v-if="sources.events.length">
      <template #title>History</template>
      <ul class="flex flex-col gap-2">
        <li v-for="(event, index) in sources.events" :key="index" class="flex gap-3">
          <span :class="{ problem: event.problem }" class="dot" />
          <div>
            <p>{{ event.message }}</p>
            <p :title="event.created_at" class="text-xs text-k-fg-50">{{ timeAgo(event.created_at) }}</p>
          </div>
        </li>
      </ul>
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Sources } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { timeAgo } from '@/utils/formatters'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import AlertBox from '@/components/ui/AlertBox.vue'
import Btn from '@/components/ui/form/Btn.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'
import LoginBrowser from '@/components/screens/settings/LoginBrowser.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess, toastWarning } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const sources = ref<Sources | null>(null)
const signingIn = ref(false)

const when = (iso: string | null) => (iso ? timeAgo(iso) : 'Never')

const look = computed(() => {
  switch (sources.value?.health.state) {
    case 'valid':
      return { alert: 'success' as const, title: 'Connected', text: 'The session works.' }
    case 'degraded':
      return {
        alert: 'warning' as const,
        title: 'Working, with hiccups',
        text: 'The last check had trouble; píxiū keeps trying.',
      }
    case 'expired':
      return {
        alert: 'danger' as const,
        title: 'Expired',
        text: 'YouTube Music signed píxiū out. Log in again; downloads that need the login wait until then.',
      }
    default:
      return { alert: 'info' as const, title: 'Not connected', text: 'Searching and downloading work without one.' }
  }
})

const fetchSources = async () => {
  try {
    sources.value = await huntingService.sources()
    signingIn.value = sources.value.login_open
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const act = async (action: () => Promise<unknown>, message?: string) => {
  try {
    await action()
    message && toastSuccess(message)
    await fetchSources()
    await huntingStore.refresh()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const signIn = () =>
  act(async () => {
    await huntingService.openLogin()
    signingIn.value = true
  })

const check = async () => {
  try {
    const health = await huntingService.validateSession()

    if (['valid', 'degraded'].includes(health.state)) {
      toastSuccess('Checked: the session works.')
    } else {
      toastWarning('Checked: the session does not work.')
    }

    await fetchSources()
    await huntingStore.refresh()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const refreshCookies = () => act(() => huntingService.refreshSession(), 'Refreshed.')

const disconnect = async () => {
  if (await showConfirmDialog('Disconnect YouTube Music? Watching liked music stops until you log in again.')) {
    await act(() => huntingService.disconnectSession(), 'Disconnected.')
  }
}

const onSignedIn = async () => {
  signingIn.value = false
  toastSuccess('Connected to YouTube Music.')
  await fetchSources()
  await huntingStore.refresh()
}

onMounted(fetchSources)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.dot {
  @apply mt-1.5 size-2.5 shrink-0 rounded-full bg-k-fg-30;

  &.problem {
    @apply bg-k-danger;
  }
}
</style>
