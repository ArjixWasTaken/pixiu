<template>
  <M3Card class="account" data-testid="account-row" variant="outlined">
    <div class="flex items-center gap-4">
      <M3Avatar :name="account.username" :size="40" />
      <div class="flex-1 min-w-0">
        <div class="flex flex-wrap items-center gap-1.5">
          <span class="m3-title-medium truncate">{{ account.username }}</span>
          <span v-if="isYou" class="pill m3-label-small">You</span>
          <span v-if="account.role === 'admin'" class="pill m3-label-small">Admin</span>
          <span v-if="account.status !== 'active'" class="pill warn m3-label-small">{{ statusLabel }}</span>
          <span v-if="account.password_change_required" class="pill m3-label-small">Temporary password</span>
        </div>
        <p class="m3-body-medium text-(--schemes-on-surface-variant) truncate">{{ details }}</p>
      </div>

      <div ref="menuAnchor" class="relative">
        <M3IconButton icon="more_vert" title="Manage this account" @click="menuOpen = !menuOpen" />
        <M3Menu v-show="menuOpen" class="menu">
          <M3MenuItem
            :label="account.role === 'admin' ? 'Remove admin' : 'Make admin'"
            icon="shield_person"
            tag="div"
            @click="choose(() => emit('toggleRole'))"
          />
          <M3MenuItem
            v-if="!isYou"
            :label="account.status === 'disabled' ? 'Turn on' : 'Turn off'"
            :icon="account.status === 'disabled' ? 'toggle_on' : 'toggle_off'"
            tag="div"
            @click="choose(() => emit('toggleStatus'))"
          />
          <M3MenuItem
            icon="password"
            label="Set a temporary password"
            tag="div"
            @click="choose(() => (settingPassword = true))"
          />
          <M3MenuItem
            v-if="!isYou"
            icon="delete"
            label="Delete account…"
            tag="div"
            @click="choose(() => emit('remove'))"
          />
        </M3Menu>
      </div>
    </div>

    <form v-if="settingPassword" class="flex gap-3 items-center mt-4" @submit.prevent="handleSubmit">
      <M3TextField
        v-model="data.password"
        autocomplete="new-password"
        class="flex-1"
        label="Temporary password"
        name="password"
        required
      />
      <M3Button type="submit">Set</M3Button>
      <M3Button variant="text" @click.prevent="settingPassword = false">Cancel</M3Button>
    </form>
  </M3Card>
</template>

<script lang="ts" setup>
import { computed, ref, useTemplateRef } from 'vue'
import { onClickOutside } from '@vueuse/core'
import type { ManagedAccount } from '@/services/adminService'
import { formatBytes, pluralize, timeAgo } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'

import M3Avatar from '@/components/m3/M3Avatar.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Menu from '@/components/m3/M3Menu.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{ account: ManagedAccount; isYou: boolean }>()
const emit = defineEmits<{
  (e: 'toggleRole'): void
  (e: 'toggleStatus'): void
  (e: 'setPassword', password: string): void
  (e: 'remove'): void
}>()

const menuOpen = ref(false)
const menuAnchor = useTemplateRef('menuAnchor')
onClickOutside(menuAnchor, () => (menuOpen.value = false))

const settingPassword = ref(false)

const statusLabel = computed(
  () =>
    ({
      pending: 'Awaiting approval',
      unverified: 'Email not confirmed',
      active: 'On',
      disabled: 'Off',
    })[props.account.status],
)

const youtubeLabel = computed(
  () =>
    ({
      none: 'no YouTube Music account',
      valid: 'YouTube Music connected',
      degraded: 'YouTube Music having trouble',
      expired: 'YouTube Music signed out',
    })[props.account.youtube_music],
)

const details = computed(() =>
  [
    props.account.email,
    `${pluralize(props.account.songs, 'song')} · ${formatBytes(props.account.bytes)}`,
    props.account.last_seen ? `seen ${timeAgo(props.account.last_seen)}` : 'never signed in',
    youtubeLabel.value,
  ]
    .filter(Boolean)
    .join(' · '),
)

const choose = (action: () => void) => {
  menuOpen.value = false
  action()
}

const { data, handleSubmit } = useForm<{ password: string }>({
  initialValues: { password: '' },
  useOverlay: false,
  validator: ({ password }) => password.length >= 8,
  onSubmit: async ({ password }) => emit('setPassword', password),
  onSuccess: () => {
    data.password = ''
    settingPassword.value = false
  },
})
</script>

<style scoped>
.account {
  padding: 16px;
}

.pill {
  padding: 2px 8px;
  border-radius: 8px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);

  &.warn {
    background: var(--schemes-error-container);
    color: var(--schemes-on-error-container);
  }
}

.menu {
  position: absolute;
  right: 0;
  top: 100%;
  z-index: 10;
}
</style>
