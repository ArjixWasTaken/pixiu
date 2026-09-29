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
      <YouTubeSidebarItem v-if="youtubeVideoTitle" data-testid="youtube" :active="isCurrentScreen('YouTube')">
        {{ youtubeVideoTitle }}
      </YouTubeSidebarItem>
      <SidebarItem
        v-if="usesPodcasts && !isDemo"
        :href="url('podcasts.index')"
        :active="isCurrentScreen('Podcasts', 'Podcast', 'Episode')"
      >
        <template #icon>
          <M3Icon name="podcasts" />
        </template>
        Podcasts
      </SidebarItem>
      <SidebarItem v-if="usesRadio" :href="url('radio-stations.index')" :active="isCurrentScreen('Radio.Stations')">
        <template #icon>
          <M3Icon name="radio" />
        </template>
        Radio
      </SidebarItem>
      <SidebarItem v-if="supportsOffline" :href="url('offline-songs')" :active="isCurrentScreen('OfflineSongs')">
        <template #icon>
          <M3Icon name="cloud_download" />
        </template>
        Available Offline
      </SidebarItem>
      <MediaBrowserMenuItem v-if="usesMediaBrowser" :active="isCurrentScreen('MediaBrowser')" />
    </ul>
  </SidebarSection>
</template>

<script lang="ts" setup>
import { unescape } from 'lodash-es'
import { computed, ref, toRef } from 'vue'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'
import { commonStore } from '@/stores/commonStore'

import M3Icon from '@/components/m3/M3Icon.vue'
import SidebarSection from '@/components/layout/main-wrapper/sidebar/SidebarSection.vue'
import SidebarSectionHeader from '@/components/layout/main-wrapper/sidebar/SidebarSectionHeader.vue'
import SidebarItem from '@/components/layout/main-wrapper/sidebar/SidebarItem.vue'
import YouTubeSidebarItem from '@/components/layout/main-wrapper/sidebar/YouTubeSidebarItem.vue'
import MediaBrowserMenuItem from '@/components/layout/main-wrapper/sidebar/MediaBrowserMenuItem.vue'

const youtubeVideoTitle = ref<string | null>(null)
const { url, isCurrentScreen } = useRouter()

const usesMediaBrowser = toRef(commonStore.state, 'uses_media_browser')
const usesPodcasts = toRef(commonStore.state, 'uses_podcasts')
const usesRadio = toRef(commonStore.state, 'uses_radio')
const { swReady, cachedSongCount } = useOfflinePlayback()
const supportsOffline = computed(() => swReady.value && cachedSongCount.value > 0)
const isDemo = window.KOEL.is_demo

eventBus.on('PLAY_YOUTUBE_VIDEO', payload => (youtubeVideoTitle.value = unescape(payload.title)))
</script>
