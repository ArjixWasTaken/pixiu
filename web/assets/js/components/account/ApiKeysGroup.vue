<template>
  <SettingGroup>
    <template #title>API keys</template>
    <template #subtitle>
      Subsonic apps that support API keys sign in with one instead of your password. Revoking a key signs its app out.
      Each browser signed in to this player has a key named “Web session”.
    </template>

    <form class="flex gap-3 items-center mb-4" @submit.prevent="handleSubmit">
      <M3TextField
        v-model="data.name"
        class="flex-1"
        label="What will use it?"
        name="name"
        placeholder="e.g. Phone"
        required
      />
      <M3Button type="submit">Create</M3Button>
    </form>

    <AlertBox v-if="created" type="success">
      <p>
        Here is the key for <strong>{{ created.name }}</strong
        >. Copy it now: píxiū shows it only this once.
      </p>
      <div class="flex items-center gap-2 mt-2">
        <code class="flex-1 font-mono text-[13px] break-all">{{ created.key }}</code>
        <M3Button variant="text" @click.prevent="copy">Copy</M3Button>
      </div>
    </AlertBox>

    <M3List class="-mx-3 py-0!">
      <M3ListItem
        v-for="key in keys"
        :key="key.id"
        :supporting="`Created ${timeAgo(key.created_at)} · ${key.last_used_at ? `used ${timeAgo(key.last_used_at)}` : 'never used'}`"
      >
        <template #leading>
          <M3Icon name="key" />
        </template>
        <template #headline>
          {{ key.name }}
          <span v-if="key.current" class="m3-label-small this-browser">This browser</span>
        </template>
        <template v-if="!key.current" #trailing>
          <M3Button class="text-(--schemes-error)!" variant="text" @click.prevent="revoke(key)">Revoke</M3Button>
        </template>
      </M3ListItem>
    </M3List>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { accountService } from '@/services/accountService'
import type { ApiKeyInfo } from '@/services/accountService'
import { timeAgo } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import AlertBox from '@/components/ui/AlertBox.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3List from '@/components/m3/M3List.vue'
import M3ListItem from '@/components/m3/M3ListItem.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const keys = ref<ApiKeyInfo[]>([])
const created = ref<{ name: string; key: string } | null>(null)

const fetchKeys = async () => {
  try {
    keys.value = await accountService.keys()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const { data, handleSubmit } = useForm<{ name: string }>({
  initialValues: { name: '' },
  validator: ({ name }) => name.trim() !== '',
  onSubmit: async ({ name }) => await accountService.createKey(name.trim()),
  onSuccess: async (key: { name: string; key: string }) => {
    created.value = key
    data.name = ''
    await fetchKeys()
  },
})

const copy = async () => {
  await navigator.clipboard.writeText(created.value!.key)
  toastSuccess('Copied.')
}

const revoke = async (key: ApiKeyInfo) => {
  if (!(await showConfirmDialog(`Revoke “${key.name}”? Whatever uses it is signed out.`))) {
    return
  }

  try {
    await accountService.revokeKey(key.id)
    await fetchKeys()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchKeys)
</script>

<style scoped>
.this-browser {
  margin-left: 6px;
  padding: 2px 8px;
  border-radius: 8px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);
}
</style>
