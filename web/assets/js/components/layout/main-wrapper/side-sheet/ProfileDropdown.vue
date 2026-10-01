<template>
  <M3MenuPopover v-model:open="open" class="flex items-center" menu-class="profile-menu">
    <template #anchor>
      <button
        :style="{ width: `${size}px`, height: `${size}px` }"
        aria-label="Account"
        class="rounded-full cursor-pointer overflow-hidden block"
        data-testid="profile-dropdown-trigger"
        type="button"
        @click="open = !open"
      >
        <M3Avatar v-if="currentUser" :name="currentUser.name" :size :src="currentUser.avatar" />
      </button>
    </template>

    <div v-if="currentUser" class="who" data-testid="profile-menu-who">
      <span class="m3-title-small block truncate">{{ currentUser.name }}</span>
      <span v-if="secondary" class="m3-body-small block truncate who-secondary">{{ secondary }}</span>
    </div>
    <template v-for="item in items" :key="item.id">
      <M3Divider class="my-2" />
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
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'

import M3Avatar from '@/components/m3/M3Avatar.vue'
import M3Divider from '@/components/m3/M3Divider.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'
import M3MenuPopover from '@/components/m3/M3MenuPopover.vue'

withDefaults(defineProps<{ size?: number }>(), { size: 40 })

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

const items = computed(() =>
  applyFilters<ContextMenuAction[]>(Filter.PROFILE_MENU_ITEMS, [
    { id: 'account', label: () => 'Account', action: () => go(`${url('settings')}?tab=account`) },
    { id: 'profile', label: () => 'Preferences', action: () => go(url('profile')) },
    { id: 'logout', label: () => 'Log out', action: () => eventBus.emit('LOG_OUT') },
    {
      id: 'about',
      label: () => (shouldNotifyNewVersion.value ? 'New version available!' : `About ${appName}`),
      action: openAbout,
    },
  ]),
)

const choose = (item: ContextMenuAction) => {
  close()
  item.action()
}
</script>

<style scoped>
:deep(.profile-menu) {
  min-width: 220px;
}

.who {
  padding: 8px 16px;
  max-width: 280px;
}

.who-secondary {
  color: var(--schemes-on-surface-variant);
}
</style>
