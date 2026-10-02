<template>
  <template v-if="!isMobile">
    <nav v-if="expanded" class="drawer" data-testid="sidebar">
      <header class="brand">
        <span class="flex items-center gap-2.5">
          <img alt="" height="28" src="/img/emblem-192.png" width="28" />
          <span class="m3-title-large text-(--schemes-on-surface)">píxiū</span>
        </span>
        <M3IconButton icon="menu_open" label="Collapse navigation" @click="expanded = false" />
      </header>
      <SearchForm :autofocus="searchOnOpen" class="search" />
      <SessionExpiredNotice />
      <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">
        <SidebarNavigation />
      </div>
      <footer class="account">
        <ProfileDropdown :size="28" class="w-full" placement="top-start" with-name />
      </footer>
    </nav>

    <M3NavigationRail
      v-else
      :items="railItems"
      :value="currentRailItem"
      class="rail shrink-0"
      @select="go(url(routeOf($event)))"
    >
      <template #top>
        <M3IconButton icon="menu" label="Open navigation" @click="expanded = true" />
        <M3IconButton icon="search" label="Search" @click="openSearch" />
      </template>
      <template #bottom>
        <ProfileDropdown :size="32" placement="right-end" />
      </template>
    </M3NavigationRail>
  </template>

  <template v-else>
    <Transition name="scrim">
      <div v-if="mobileShowing" ref="scrim" class="scrim" data-testid="sidebar-scrim" @click="close" />
    </Transition>
    <Transition name="drawer">
      <nav
        v-if="mobileShowing"
        ref="drawer"
        class="modal-drawer"
        data-testid="sidebar"
        @pointercancel="onPointerUp"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
      >
        <header class="flex items-center gap-2.5 pt-3 pr-2 pb-1 pl-7">
          <img alt="" height="32" src="/img/emblem-192.png" width="32" />
          <span class="m3-title-large flex-1 text-(--schemes-on-surface)">píxiū</span>
          <M3IconButton icon="close" label="Close navigation" @click="close" />
        </header>
        <SessionExpiredNotice />
        <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden overscroll-contain">
          <SidebarNavigation />
        </div>
      </nav>
    </Transition>
  </template>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'
import type { RouteName } from '@/config/routes'
import { eventBus } from '@/utils/eventBus'
import { useUserStorage } from '@/composables/useUserStorage'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'
import { useBackToClose } from '@/composables/useBackToClose'
import { requireInjection } from '@/utils/helpers'
import { ModalKey } from '@/config/symbols'
import { useHuntingStore } from '@/stores/huntingStore'
import { usePolicies } from '@/composables/usePolicies'
import type { M3NavItem } from '@/components/m3/navigation'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3NavigationRail from '@/components/m3/M3NavigationRail.vue'
import ProfileDropdown from '@/components/layout/main-wrapper/side-sheet/ProfileDropdown.vue'
import SearchForm from '@/components/ui/SearchForm.vue'
import SessionExpiredNotice from './SessionExpiredNotice.vue'
import SidebarNavigation from './SidebarNavigation.vue'

const { onRouteChanged, isCurrentScreen, go, url } = useRouter()
const { isMobile } = useViewport()
const huntingStore = useHuntingStore()
const { currentUserCan } = usePolicies()

const collapsed = useUserStorage('sidebar-collapsed', false)
const expanded = computed({ get: () => !collapsed.value, set: value => (collapsed.value = !value) })

/** From the rail (its button, or the F key): open the drawer, where the search field is. */
const searchOnOpen = ref(false)

const openSearch = () => {
  searchOnOpen.value = true
  expanded.value = true
}

const searchFromRail = () => !isMobile.value && !expanded.value && openSearch()

watch(expanded, value => value || (searchOnOpen.value = false))

onMounted(() => eventBus.on('FOCUS_SEARCH_FIELD', searchFromRail))
onBeforeUnmount(() => eventBus.off('FOCUS_SEARCH_FIELD', searchFromRail))

type RailItem = M3NavItem & {
  route: RouteName
  screens: ScreenName[]
  /** What the drawer's item shows beside it (failed jobs, uploads to review…). */
  count?: () => number
  visible?: () => boolean
}

// Every place the drawer leads to, but its playlists (Open navigation lists those).
const rail: RailItem[] = [
  { id: 'Home', label: 'Home', icon: 'home', route: 'home', screens: ['Home'] },
  { id: 'Songs', label: 'Songs', icon: 'music_note', route: 'songs.index', screens: ['Songs'] },
  { id: 'Albums', label: 'Albums', icon: 'album', route: 'albums.index', screens: ['Albums', 'Album'] },
  { id: 'Artists', label: 'Artists', icon: 'artist', route: 'artists.index', screens: ['Artists', 'Artist'] },
  { id: 'Genres', label: 'Genres', icon: 'category', route: 'genres.index', screens: ['Genres', 'Genre'] },
  { id: 'Favorites', label: 'Favorites', icon: 'favorite', route: 'favorites', screens: ['Favorites'] },
  { id: 'Recent', label: 'Recent', icon: 'history', route: 'recently-played', screens: ['RecentlyPlayed'] },
  { id: 'Hunt', label: 'Discover', icon: 'travel_explore', route: 'hunt', screens: ['Hunt'] },
  { id: 'Watches', label: 'Watches', icon: 'visibility', route: 'watches', screens: ['Watches'] },
  {
    id: 'Jobs',
    label: 'Jobs',
    icon: 'checklist',
    route: 'jobs',
    screens: ['Jobs'],
    count: () => huntingStore.state.jobs.failed,
  },
  {
    id: 'Uploads',
    label: 'Uploads',
    icon: 'upload',
    route: 'upload',
    screens: ['Upload'],
    count: () => huntingStore.state.offerings,
    visible: () => currentUserCan.uploadSongs(),
  },
  {
    id: 'Orphans',
    label: 'Orphans',
    icon: 'cleaning_services',
    route: 'orphans',
    screens: ['Orphans'],
    count: () => huntingStore.state.orphans,
  },
  {
    id: 'Settings',
    label: 'Settings',
    icon: 'settings',
    route: 'settings',
    screens: ['Settings'],
    count: () => huntingStore.state.registrations,
  },
]

const railItems = computed(() =>
  rail
    .filter(item => item.visible?.() ?? true)
    .map(({ count, visible: _visible, ...item }) => ({
      ...item,
      href: url(item.route),
      badge: count?.() || undefined,
    })),
)
const currentRailItem = computed(() => rail.find(item => isCurrentScreen(...item.screens))?.id)
const routeOf = (item: M3NavItem) => rail.find(({ id }) => id === item.id)!.route

const mobileShowing = ref(false)
const close = () => (mobileShowing.value = false)

useBackToClose(mobileShowing, close)

// A dialog opened from the drawer (a new playlist, say) takes the screen: the drawer goes.
const modal = requireInjection(ModalKey)
watch(
  () => modal.value.component,
  component => component && close(),
)

onRouteChanged(close)

/**
 * The mobile menu button opens the drawer. Items emit this too after
 * navigating, so only opening is honored here: the route change closes it.
 */
eventBus.on('TOGGLE_SIDEBAR', () => {
  if (isMobile.value && !mobileShowing.value) {
    mobileShowing.value = true
  }
})

// Swiping the drawer to the left closes it.
const drawer = useTemplateRef('drawer')
const scrim = useTemplateRef('scrim')
let drag: { x: number; y: number; t: number; dx: number; on: boolean; id: number } | null = null

const onPointerDown = (event: PointerEvent) => {
  drag = { x: event.clientX, y: event.clientY, t: Date.now(), dx: 0, on: false, id: event.pointerId }
}

const onPointerMove = (event: PointerEvent) => {
  if (!drag || !drawer.value) {
    return
  }

  const dx = event.clientX - drag.x

  if (!drag.on && Math.abs(dx) > 8 && Math.abs(dx) > Math.abs(event.clientY - drag.y)) {
    drag.on = true
    drawer.value.setPointerCapture?.(drag.id)
  }

  if (!drag.on) {
    return
  }

  drag.dx = Math.min(0, dx)
  drawer.value.style.transition = 'none'
  drawer.value.style.transform = `translateX(${drag.dx}px)`
  scrim.value && (scrim.value.style.opacity = String(Math.max(0, 1 + drag.dx / drawer.value.offsetWidth)))
}

const onPointerUp = () => {
  const current = drag
  drag = null

  if (!current?.on || !drawer.value) {
    return
  }

  const velocity = current.dx / Math.max(1, Date.now() - current.t)
  drawer.value.style.transition = ''
  drawer.value.style.transform = ''
  scrim.value && (scrim.value.style.opacity = '')

  if (current.dx < -drawer.value.offsetWidth * 0.3 || velocity < -0.5) {
    close()
  }
}
</script>

<style scoped>
.drawer {
  position: relative;
  display: flex;
  flex-direction: column;
  width: var(--m3-nav-width);
  flex-shrink: 0;
  min-height: 0;
  background: var(--schemes-surface-container-low);
}

.rail {
  background: var(--schemes-surface-container-low);
}

.brand {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 8px 4px 24px;
}

.search {
  margin: 8px 12px;
}

.account {
  padding: 8px 12px 12px;
}

.scrim {
  position: fixed;
  inset: 0;
  z-index: 900;
  /* Dark schemes hide a lighter scrim against their dark pages. */
  background: color-mix(in srgb, var(--schemes-scrim) 50%, transparent);
  transition: opacity 200ms linear;
}

.modal-drawer {
  position: fixed;
  top: 0;
  bottom: 0;
  left: 0;
  z-index: 901;
  display: flex;
  flex-direction: column;
  width: min(360px, calc(100% - 28px));
  overflow: hidden;
  border-radius: 0 16px 16px 0;
  background: var(--schemes-surface-container-low);
  touch-action: pan-y;
  transition: transform 200ms var(--m3-ease);
}

.scrim-enter-from,
.scrim-leave-to {
  opacity: 0;
}

.drawer-enter-from,
.drawer-leave-to {
  transform: translateX(-105%);
}
</style>
