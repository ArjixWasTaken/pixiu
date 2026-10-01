<template>
  <Overlay ref="overlay" />
  <DialogBox ref="dialog" />
  <MessageToaster ref="toaster" />
  <GlobalEventListeners />
  <OfflineNotification v-if="!online" />
  <UpdateNotification />

  <ChangePasswordRequired v-if="layout === 'default' && initialized && mustChangePassword" />

  <main
    v-if="layout === 'default' && initialized && !mustChangePassword"
    class="relative h-dvh w-full flex flex-col"
    @dragend="onDragEnd"
    @dragleave="onDragLeave"
    @dragover="onDragOver"
    @drop="onDrop"
  >
    <HotkeyListener />
    <MainWrapper />
    <AppFooter />
    <MobileNavigationBar v-if="isMobile" />
    <DropZone v-show="showDropZone" @close="showDropZone = false" />
  </main>

  <Auth v-if="layout === 'auth'" @logged-in="triggerAppInitialization" />
  <EmailLink v-if="layout === 'email-link'" />
  <SsoComplete v-if="layout === 'sso'" />

  <AppInitializer v-if="authenticated" @error="onInitError" @success="onInitSuccess" />

  <ContextMenu />
</template>

<script lang="ts" setup>
import { defineAsyncComponent } from '@/utils/helpers'
import { computed, onMounted, provide, ref, shallowRef, watch } from 'vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { useQueueStore } from '@/stores/queueStore'
import { authService } from '@/services/authService'
import {
  ContextMenuKey,
  CurrentStreamableKey,
  DialogBoxKey,
  MessageToasterKey,
  ModalKey,
  OverlayKey,
} from '@/config/symbols'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'
import { useUserStore } from '@/stores/userStore'
import { activeRouter } from '@/router'

import DialogBox from '@/components/ui/DialogBox.vue'
import MessageToaster from '@/components/ui/message-toaster/MessageToaster.vue'
import Overlay from '@/components/ui/Overlay.vue'
import OfflineNotification from '@/components/ui/OfflineNotification.vue'
import UpdateNotification from '@/components/ui/UpdateNotification.vue'

// Do not dynamic-import app footer, as it contains the <audio> element
// that is necessary to properly initialize the playService and equalizer.
import AppFooter from '@/components/layout/app-footer/index.vue'
import MobileNavigationBar from '@/components/layout/MobileNavigationBar.vue'

// GlobalEventListener must NOT be lazy-loaded, so that it can handle LOG_OUT event properly.
import GlobalEventListeners from '@/components/utils/GlobalEventListeners.vue'
import AppInitializer from '@/components/utils/AppInitializer.vue'
import ContextMenu from '@/components/ui/context-menu/ContextMenu.vue'

const queueStore = useQueueStore()
const userStore = useUserStore()

const HotkeyListener = defineAsyncComponent(() => import('@/components/utils/HotkeyListener.vue'))
const Auth = defineAsyncComponent(() => import('@/components/auth/Auth.vue'))
const MainWrapper = defineAsyncComponent(() => import('@/components/layout/main-wrapper/index.vue'))
const DropZone = defineAsyncComponent(() => import('@/components/ui/upload/DropZone.vue'))
const ChangePasswordRequired = defineAsyncComponent(() => import('@/components/account/ChangePasswordRequired.vue'))
const EmailLink = defineAsyncComponent(() => import('@/components/auth/EmailLink.vue'))
const SsoComplete = defineAsyncComponent(() => import('@/components/auth/SsoComplete.vue'))

const overlay = ref<InstanceType<typeof Overlay>>()
const dialog = ref<InstanceType<typeof DialogBox>>()
const toaster = ref<InstanceType<typeof MessageToaster>>()
const currentStreamable = ref<Streamable>()
const showDropZone = ref(false)

const { isCurrentScreen, startGuarding } = useRouter()
const { online } = useNetworkStatus()
const { isMobile } = useViewport()

const authenticated = ref(false)
const initialized = ref(false)
const currentRoute = computed(() => activeRouter().currentRoute.value)

const triggerAppInitialization = () => (authenticated.value = true)
const onInitError = () => (authenticated.value = false)

const onInitSuccess = async () => {
  initialized.value = true
  startGuarding()
}

/** Signed in with a temporary password: they pick their own first. */
const mustChangePassword = computed(() => Boolean(userStore.state.current?.password_change_required))

const layout = computed(() => {
  if (currentRoute.value.meta.layout) {
    return currentRoute.value.meta.layout
  }

  return authenticated.value ? 'default' : 'auth'
})

onMounted(async () => {
  // Add an ugly mac/non-mac class for OS-targeting styles.
  document.documentElement.classList.add(navigator.userAgent.includes('Mac') ? 'mac' : 'non-mac')

  await activeRouter().isReady()

  if (currentRoute.value.meta.public) {
    // If the route is public (sign-in, email links etc.) we don't need to check for authentication.
    return
  }

  // If the user is authenticated via a proxy, we have the token in the window object.
  // Simply forward it to the authService and continue with the normal flow.
  if (window.KOEL.auth_token) {
    authService.setTokensUsingCompositeToken(window.KOEL.auth_token)
  }

  // The app has just been initialized, check if we can get the user data with an already existing token
  if (authService.hasApiToken()) {
    triggerAppInitialization()
  }
})

const onDragOver = (e: DragEvent) => {
  showDropZone.value = Boolean(e.dataTransfer?.types.includes('Files')) && !isCurrentScreen('Upload')
}

watch(
  () => queueStore.current,
  song => (currentStreamable.value = song),
)

const onDragEnd = () => (showDropZone.value = false)

const onDragLeave = (e: MouseEvent) => {
  if ((e.currentTarget as Node)?.contains?.(e.relatedTarget as Node)) {
    return
  }

  showDropZone.value = false
}

const onDrop = () => (showDropZone.value = false)

provide(OverlayKey, overlay)
provide(DialogBoxKey, dialog)
provide(MessageToasterKey, toaster)
provide(CurrentStreamableKey, currentStreamable)

provide(
  ContextMenuKey,
  shallowRef({
    component: null,
    position: { top: 0, left: 0 },
  }),
)

provide(
  ModalKey,
  shallowRef({
    component: null,
  }),
)
</script>

<style lang="postcss">
@reference '@css/app.pcss';
#dragGhost {
  @apply hidden py-2 pl-8 pr-3 rounded-md text-base fixed bg-(--schemes-surface-container) border border-(--schemes-outline-variant)
  text-(--schemes-on-surface) pointer-events-none z-50 whitespace-nowrap;
}

#copyArea {
  @apply absolute -left-full bottom-px w-px h-px no-hover:hidden;
}
</style>
