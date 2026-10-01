<template>
  <SidebarSection>
    <template #header>
      <SidebarSectionHeader>Manage</SidebarSectionHeader>
    </template>

    <ul class="menu">
      <SidebarItem
        v-for="item in visibleItems"
        :key="item.route"
        :href="url(item.route)"
        :active="isCurrentScreen(...item.screens)"
        :icon="item.busy ? 'progress_activity' : item.icon"
        :spin="item.busy"
      >
        <template v-if="item.badgeLabel" #badge>{{ item.badgeLabel }}</template>
        {{ item.label }}
        <span aria-live="polite" class="sr-only">{{ item.busy ? 'in progress' : '' }}</span>
      </SidebarItem>
    </ul>
  </SidebarSection>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import type { RouteName } from '@/config/routes'
import { useRouter } from '@/composables/useRouter'
import { usePolicies } from '@/composables/usePolicies'
import { uploadService } from '@/services/uploadService'
import { useHuntingStore } from '@/stores/huntingStore'
import SidebarSection from '@/components/layout/main-wrapper/sidebar/SidebarSection.vue'
import SidebarSectionHeader from '@/components/layout/main-wrapper/sidebar/SidebarSectionHeader.vue'
import SidebarItem from '@/components/layout/main-wrapper/sidebar/SidebarItem.vue'

const huntingStore = useHuntingStore()

export interface ManageSidebarItem {
  label: string
  /** A Material Symbols name. */
  icon: string
  route: RouteName
  screens: ScreenName[]
  visible: () => boolean
  badge?: () => string | null
  isBusy?: () => boolean
}

const { url, isCurrentScreen } = useRouter()
const { currentUserCan } = usePolicies()

const items = computed<ManageSidebarItem[]>(() => [
  {
    label: 'Discover',
    icon: 'travel_explore',
    route: 'hunt',
    screens: ['Hunt'],
    visible: () => true,
  },
  {
    label: 'Watches',
    icon: 'visibility',
    route: 'watches',
    screens: ['Watches'],
    visible: () => true,
  },
  {
    label: 'Jobs',
    icon: 'checklist',
    route: 'jobs',
    screens: ['Jobs'],
    visible: () => true,
    badge: () => (huntingStore.state.jobs.failed ? String(huntingStore.state.jobs.failed) : null),
    isBusy: () => huntingStore.state.jobs.running > 0,
  },
  {
    label: 'Uploads',
    icon: 'upload',
    route: 'upload',
    screens: ['Upload'],
    visible: () => currentUserCan.uploadSongs(),
    badge: () => (huntingStore.state.offerings ? String(huntingStore.state.offerings) : null),
    isBusy: () => uploadService.getUnfinishedFiles().length > 0,
  },
  {
    label: 'Orphans',
    icon: 'cleaning_services',
    route: 'orphans',
    screens: ['Orphans'],
    visible: () => true,
    badge: () => (huntingStore.state.orphans ? String(huntingStore.state.orphans) : null),
  },
  {
    label: 'Settings',
    icon: 'settings',
    route: 'settings',
    screens: ['Settings'],
    // Everyone has their account and library settings there.
    visible: () => true,
    badge: () => (huntingStore.state.registrations ? String(huntingStore.state.registrations) : null),
  },
])

const visibleItems = computed(() =>
  items.value
    .filter(item => item.visible())
    .map(item => ({ ...item, badgeLabel: item.badge?.() ?? null, busy: item.isBusy?.() ?? false })),
)
</script>
