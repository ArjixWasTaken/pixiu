<template>
  <M3MenuPopover v-model:open="open" :placement class="flex items-center" :min-width="220">
    <template #anchor>
      <button
        v-if="withName"
        aria-label="Account"
        class="account-row m3-state m3-label-large"
        data-testid="profile-dropdown-trigger"
        type="button"
      >
        <M3Avatar v-if="currentUser" :name="currentUser.name" :size :src="currentUser.avatar" />
        <span class="flex-1 min-w-0 truncate text-left">{{ currentUser?.name }}</span>
        <M3Icon :size="20" name="unfold_more" />
      </button>
      <button
        v-else
        :style="{ width: `${Math.max(size, 40)}px`, height: `${Math.max(size, 40)}px` }"
        aria-label="Account"
        class="rounded-full cursor-pointer grid place-items-center shrink-0"
        data-testid="profile-dropdown-trigger"
        type="button"
      >
        <M3Avatar v-if="currentUser" :name="currentUser.name" :size :src="currentUser.avatar" />
      </button>
    </template>

    <div v-if="currentUser" class="who" data-testid="profile-menu-who">
      <span class="m3-title-small block truncate">{{ currentUser.name }}</span>
      <span v-if="secondary" class="m3-body-small block truncate who-secondary">{{ secondary }}</span>
    </div>
    <!-- Dividers between groups: the account, about píxiū, signing out. -->
    <template v-for="(item, index) in items" :key="item.id">
      <M3Divider v-if="index === 0 || item.id === 'about' || item.id === 'logout'" class="my-2" />
      <M3MenuItem :data-testid="`profile-menu-${item.id}`" :label="item.label()" tag="div" @click="choose(item)" />
    </template>
  </M3MenuPopover>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { useAuthorization } from '@/composables/useAuthorization'
import { useRouter } from '@/composables/useRouter'
import { useNewVersionNotification } from '@/composables/useNewVersionNotification'
import { useBranding } from '@/composables/useBranding'
import { useModal } from '@/composables/useModal'
import { defineAsyncComponent } from '@/utils/helpers'

import M3Avatar from '@/components/m3/M3Avatar.vue'
import M3Divider from '@/components/m3/M3Divider.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'
import M3MenuPopover from '@/components/m3/M3MenuPopover.vue'

withDefaults(
  defineProps<{
    size?: number
    /** A full-width row with the account's name, as at the foot of the sidebar. */
    withName?: boolean
    placement?: 'bottom-end' | 'top-start' | 'right-end'
  }>(),
  { size: 40, withName: false, placement: 'bottom-end' },
)

const AboutKoelModal = defineAsyncComponent(() => import('@/components/meta/AboutKoelModal.vue'))

const { go, url } = useRouter()
const { currentUser } = useAuthorization()
const { shouldNotifyNewVersion } = useNewVersionNotification()
const { name: appName } = useBranding()
const { openModal } = useModal()

const open = ref(false)

/** Who is signed in, beyond their name: their login, or their email. */
const secondary = computed(() => {
  const user = currentUser.value
  if (!user) {
    return ''
  }
  return user.username && user.username !== user.name ? user.username : user.email
})

const close = () => (open.value = false)

const openAbout = () => openModal<'ABOUT_KOEL'>(AboutKoelModal)

const items = computed<ContextMenuAction[]>(() => [
  { id: 'account', label: () => 'Account', action: () => go(`${url('settings')}#account`) },
  { id: 'profile', label: () => 'Preferences', action: () => go(`${url('settings')}#preferences`) },
  {
    id: 'about',
    label: () => (shouldNotifyNewVersion.value ? 'New version available!' : `About ${appName}`),
    action: openAbout,
  },
  { id: 'logout', label: () => 'Sign out', action: () => eventBus.emit('LOG_OUT') },
])

const choose = (item: ContextMenuAction) => {
  close()
  item.action()
}
</script>

<style scoped>
.who {
  padding: 8px 16px;
  max-width: 280px;
}

.who-secondary {
  color: var(--schemes-on-surface-variant);
}

.account-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  height: 48px;
  padding: 0 12px;
  border-radius: 12px;
  color: var(--schemes-on-surface);
  cursor: pointer;
}
</style>
