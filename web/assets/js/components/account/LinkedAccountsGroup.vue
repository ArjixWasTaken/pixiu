<template>
  <SettingGroup v-if="sso || links.length" data-testid="linked-accounts">
    <template #title>Single sign-on</template>
    <template #subtitle>
      {{
        sso
          ? `Link your ${sso.name} account, and sign in with it instead of your password.`
          : 'Single sign-on is not set up any more; your links wait in case it comes back.'
      }}
    </template>

    <ul v-if="links.length" class="flex flex-col gap-2 mb-4">
      <li v-for="link in links" :key="link.id" class="flex items-center gap-3">
        <M3Icon name="key" />
        <span class="flex-1 min-w-0">
          <span class="block m3-body-large text-(--schemes-on-surface) truncate">
            {{ link.provider }}<template v-if="link.email"> · {{ link.email }}</template>
          </span>
          <span class="block m3-body-medium text-(--schemes-on-surface-variant)">
            {{ link.last_login_at ? `Last used ${timeAgo(link.last_login_at)}` : 'Not used to sign in yet' }}
          </span>
        </span>
        <M3Button variant="text" @click="unlink(link)">Unlink</M3Button>
      </li>
    </ul>

    <M3Button v-if="sso && !linked" :disabled="leaving" icon="link" @click="link">
      Link your {{ sso.name }} account
    </M3Button>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { accountService } from '@/services/accountService'
import type { LinkedAccount } from '@/services/accountService'
import { authService } from '@/services/authService'
import { timeAgo } from '@/utils/formatters'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useRouter } from '@/composables/useRouter'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { getRouteParam } = useRouter()
const { showConfirmDialog, showErrorDialog } = useDialogBox()
const { toastSuccess, toastWarning } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const sso = ref<{ name: string } | null>(null)
const links = ref<LinkedAccount[]>([])
const leaving = ref(false)

/** Whether an account at today's provider is linked. */
const linked = computed(() => links.value.some(link => link.provider === sso.value?.name))

const link = async () => {
  leaving.value = true
  try {
    location.assign(await accountService.linkIdentity())
  } catch (error: unknown) {
    leaving.value = false
    handleHttpError(error)
  }
}

const unlink = async (account: LinkedAccount) => {
  if (!(await showConfirmDialog(`Unlink your ${account.provider} account? You sign in with your password then.`))) {
    return
  }

  try {
    await accountService.unlinkIdentity(account.id)
    links.value = links.value.filter(other => other.id !== account.id)
    toastSuccess(`Your ${account.provider} account is unlinked.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

/** Says how linking went, when the provider sent the browser back here. */
const reportLinking = () => {
  const provider = sso.value?.name ?? 'single sign-on'
  const linkedNow = getRouteParam('linked')
  const problem = getRouteParam('link_error')

  if (!linkedNow && !problem) {
    return
  }

  if (linkedNow) {
    toastSuccess(`Your ${provider} account is linked: sign in with it from now on.`)
  } else if (problem === 'taken') {
    showErrorDialog(`That ${provider} account is linked to another píxiū account already.`, 'Not linked')
  } else if (problem === 'denied') {
    toastWarning('Linking was cancelled.')
  } else if (problem === 'expired') {
    showErrorDialog('Linking took too long, or started in another browser. Try again.', 'Not linked')
  } else {
    showErrorDialog(`Linking your ${provider} account did not work. Try again later.`, 'Not linked')
  }

  // Said once: a reload should not say it again.
  history.replaceState(history.state, '', `${location.pathname}?tab=account`)
}

onMounted(async () => {
  try {
    ;[sso.value, links.value] = await Promise.all([
      authService.status().then(status => status.sso),
      accountService.identities(),
    ])
  } catch (error: unknown) {
    handleHttpError(error)
  }

  reportLinking()
})
</script>
