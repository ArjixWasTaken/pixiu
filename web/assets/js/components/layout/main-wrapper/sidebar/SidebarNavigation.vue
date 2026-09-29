<template>
  <div class="sidebar-navigation">
    <ul>
      <SidebarItem :active="isCurrentScreen('Home')" :href="url('home')" icon="home">Home</SidebarItem>
    </ul>
    <SidebarYourLibrarySection />
    <SidebarPlaylistsSection />
    <SidebarManageSection v-if="showManageOptions" />
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRouter } from '@/composables/useRouter'
import { usePolicies } from '@/composables/usePolicies'

import SidebarItem from './SidebarItem.vue'
import SidebarManageSection from './SidebarManageSection.vue'
import SidebarPlaylistsSection from './SidebarPlaylistsSection.vue'
import SidebarYourLibrarySection from './SidebarYourLibrarySection.vue'

const { url, isCurrentScreen } = useRouter()
const { currentUserCan } = usePolicies()

const showManageOptions = computed(
  () => currentUserCan.manageSettings() || currentUserCan.manageUsers() || currentUserCan.uploadSongs(),
)
</script>

<style scoped>
.sidebar-navigation {
  padding: 0 12px 12px;
}
</style>
