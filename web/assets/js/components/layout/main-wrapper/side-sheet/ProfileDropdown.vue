<template>
  <div ref="containerEl" class="relative flex items-center">
    <button
      :style="{ width: `${size}px`, height: `${size}px` }"
      aria-label="Account"
      class="rounded-full cursor-pointer overflow-hidden block"
      data-testid="profile-dropdown-trigger"
      type="button"
      @click.stop="open = !open"
    >
      <M3Avatar v-if="currentUser" :name="currentUser.name" :size :src="currentUser.avatar" />
    </button>

    <M3Menu v-if="open" v-koel-focus class="menu" tabindex="0" @keydown.esc="open = false">
      <template v-for="(item, index) in items" :key="item.id">
        <M3Divider v-if="index" class="my-2" />
        <M3MenuItem :data-testid="`profile-menu-${item.id}`" :label="item.label()" tag="div" @click="choose(item)" />
      </template>
    </M3Menu>
  </div>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
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
import M3Menu from '@/components/m3/M3Menu.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'

withDefaults(defineProps<{ size?: number }>(), { size: 40 })

const AboutKoelModal = defineAsyncComponent(() => import('@/components/meta/AboutKoelModal.vue'))

const { go, url } = useRouter()
const { currentUser } = useAuthorization()
const { shouldNotifyNewVersion } = useNewVersionNotification()
const { name: appName } = useBranding()
const { openModal } = useModal()

const containerEl = ref<HTMLDivElement>()
const open = ref(false)

const close = () => (open.value = false)

const openAbout = () => openModal<'ABOUT_KOEL'>(AboutKoelModal)

const items = computed(() =>
  applyFilters<ContextMenuAction[]>(Filter.PROFILE_MENU_ITEMS, [
    { id: 'account', label: () => 'Account', action: () => go(`${url('settings')}?tab=account`) },
    { id: 'profile', label: () => 'Preferences', action: () => go(url('profile')) },
    { id: 'logout', label: () => 'Log Out', action: () => eventBus.emit('LOG_OUT') },
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

const onClickOutside = (e: MouseEvent) => {
  if (open.value && containerEl.value && !containerEl.value.contains(e.target as Node)) {
    close()
  }
}

onMounted(() => document.addEventListener('click', onClickOutside))
onBeforeUnmount(() => document.removeEventListener('click', onClickOutside))
</script>

<style scoped>
.menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 50;
  min-width: 200px;
}
</style>
