<template>
  <div class="flex flex-col gap-8 md:w-2/3">
    <SettingGroup>
      <template #title>API keys</template>
      <template #subtitle>
        Subsonic apps that support API keys sign in with one instead of your password. Revoking a key signs its app out.
        Each browser signed in to this player has a key named “Web session”.
      </template>

      <form class="flex gap-2 mb-6" @submit.prevent="handleSubmit">
        <TextInput
          v-model="data.name"
          class="flex-1"
          name="name"
          placeholder="What will use it, e.g. “Phone”"
          required
        />
        <Btn type="submit">Create</Btn>
      </form>

      <AlertBox v-if="created" type="success">
        <p>
          Here is the key for <strong>{{ created.name }}</strong
          >. Copy it now: píxiū shows it only this once.
        </p>
        <div class="flex items-center gap-2 mt-2">
          <code class="flex-1 font-mono text-sm break-all">{{ created.key }}</code>
          <Btn size="small" variant="ghost" @click.prevent="copy">Copy</Btn>
        </div>
      </AlertBox>

      <ul class="divide-y divide-k-fg-5">
        <li v-for="key in keys" :key="key.id" class="flex items-center gap-4 py-2">
          <div class="flex-1 min-w-0">
            <p class="truncate">
              {{ key.name }}
              <span v-if="key.current" class="text-xs text-k-fg-50 uppercase ml-1">This browser</span>
            </p>
            <p class="text-xs text-k-fg-50">
              Created {{ timeAgo(key.created_at) }} ·
              {{ key.last_used_at ? `used ${timeAgo(key.last_used_at)}` : 'never used' }}
            </p>
          </div>
          <Btn v-if="!key.current" size="small" variant="destructive" @click.prevent="revoke(key)">Revoke</Btn>
        </li>
      </ul>
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { ApiKeyInfo } from '@/services/huntingService'
import { timeAgo } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import AlertBox from '@/components/ui/AlertBox.vue'
import Btn from '@/components/ui/form/Btn.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const keys = ref<ApiKeyInfo[]>([])
const created = ref<{ name: string; key: string } | null>(null)

const fetchKeys = async () => {
  try {
    keys.value = (await huntingService.settings()).keys
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const { data, handleSubmit } = useForm<{ name: string }>({
  initialValues: { name: '' },
  validator: ({ name }) => name.trim() !== '',
  onSubmit: async ({ name }) => await huntingService.createKey(name.trim()),
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
    await huntingService.revokeKey(key.id)
    await fetchKeys()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchKeys)
</script>
