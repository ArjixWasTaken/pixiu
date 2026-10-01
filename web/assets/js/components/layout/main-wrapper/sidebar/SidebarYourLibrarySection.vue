<template>
  <SidebarSection>
    <template #header>
      <SidebarSectionHeader>Your library</SidebarSectionHeader>
    </template>

    <ul class="menu">
      <SidebarItem :href="url('songs.index')" :active="isCurrentScreen('Songs')" icon="music_note">
        All songs
      </SidebarItem>
      <SidebarItem :href="url('albums.index')" :active="isCurrentScreen('Albums', 'Album')" icon="album">
        Albums
      </SidebarItem>
      <SidebarItem :href="url('artists.index')" :active="isCurrentScreen('Artists', 'Artist')" icon="artist">
        Artists
      </SidebarItem>
      <SidebarItem :href="url('genres.index')" :active="isCurrentScreen('Genres', 'Genre')" icon="category">
        Genres
      </SidebarItem>
      <SidebarItem v-if="supportsOffline" :href="url('offline-songs')" :active="isCurrentScreen('OfflineSongs')">
        <template #icon>
          <M3Icon name="cloud_download" />
        </template>
        Available offline
      </SidebarItem>
    </ul>
  </SidebarSection>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { useRouter } from '@/composables/useRouter'

import M3Icon from '@/components/m3/M3Icon.vue'
import SidebarSection from '@/components/layout/main-wrapper/sidebar/SidebarSection.vue'
import SidebarSectionHeader from '@/components/layout/main-wrapper/sidebar/SidebarSectionHeader.vue'
import SidebarItem from '@/components/layout/main-wrapper/sidebar/SidebarItem.vue'

const { url, isCurrentScreen } = useRouter()

const { swReady, cachedSongCount } = useOfflinePlayback()
const supportsOffline = computed(() => swReady.value && cachedSongCount.value > 0)
</script>
