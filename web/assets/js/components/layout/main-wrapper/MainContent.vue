<template>
  <section id="mainContent" class="main-content" data-testid="main-content">
    <TopBar />
    <NowPlayingPanel v-if="!isMobile" />
    <div class="screens">
      <!--
      Most of the views are render-expensive and have their own UI states (viewport/scroll position), e.g. the playable
      lists), so we use v-show.
      For those that don't need to maintain their own UI state, we use v-if to avoid rendering them when not needed.
    -->
      <HomeScreen v-if="screenLoaded('Home')" v-show="screen === 'Home'" />
      <QueueScreen v-if="screenLoaded('Queue')" v-show="screen === 'Queue'" />
      <AllSongsScreen v-if="screenLoaded('Songs')" v-show="screen === 'Songs'" />
      <AlbumListScreen v-if="screenLoaded('Albums')" v-show="screen === 'Albums'" />
      <ArtistListScreen v-if="screenLoaded('Artists')" v-show="screen === 'Artists'" />
      <PlaylistScreen v-if="screenLoaded('Playlist')" v-show="screen === 'Playlist'" />
      <FavoritesScreen v-if="screenLoaded('Favorites')" v-show="screen === 'Favorites'" />
      <RecentlyPlayedScreen v-if="screenLoaded('RecentlyPlayed')" v-show="screen === 'RecentlyPlayed'" />
      <OfflineSongsScreen v-if="screenLoaded('OfflineSongs')" v-show="screen === 'OfflineSongs'" />
      <UploadScreen v-if="screenLoaded('Upload')" v-show="screen === 'Upload'" />
      <SearchExcerptsScreen v-if="screenLoaded('Search.Excerpt')" v-show="screen === 'Search.Excerpt'" />
      <GenreScreen v-if="screenLoaded('Genre')" v-show="screen === 'Genre'" />
      <GenreListScreen v-if="screenLoaded('Genres')" v-show="screen === 'Genres'" />

      <SearchSongResultsScreen v-if="screen === 'Search.Playables'" />
      <AlbumScreen v-if="screen === 'Album'" />
      <ArtistScreen v-if="screen === 'Artist'" />
      <SettingsScreen v-if="screen === 'Settings'" />
      <HuntScreen v-if="screenLoaded('Hunt')" v-show="screen === 'Hunt'" />
      <WatchesScreen v-if="screen === 'Watches'" />
      <JobsScreen v-if="screen === 'Jobs'" />
      <OrphansScreen v-if="screen === 'Orphans'" />
      <NotFoundScreen v-if="screen === '404'" />

      <template v-for="(component, name) in addedScreens" :key="name">
        <component :is="component" v-if="screen === name" />
      </template>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { onMounted, reactive, ref } from 'vue'
import type { Component } from 'vue'
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'
import { defineAsyncComponent } from '@/utils/helpers'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'

import TopBar from '@/components/layout/main-wrapper/TopBar.vue'
import NowPlayingPanel from '@/components/layout/now-playing/NowPlayingPanel.vue'

const AlbumListScreen = defineAsyncComponent(() => import('@/components/screens/AlbumListScreen.vue'))
const AlbumScreen = defineAsyncComponent(() => import('@/components/screens/AlbumScreen.vue'))
const AllSongsScreen = defineAsyncComponent(() => import('@/components/screens/AllSongsScreen.vue'))
const ArtistListScreen = defineAsyncComponent(() => import('@/components/screens/ArtistListScreen.vue'))
const ArtistScreen = defineAsyncComponent(() => import('@/components/screens/ArtistScreen.vue'))
const FavoritesScreen = defineAsyncComponent(() => import('@/components/screens/FavoritesScreen.vue'))
const GenreListScreen = defineAsyncComponent(() => import('@/components/screens/GenreListScreen.vue'))
const GenreScreen = defineAsyncComponent(() => import('@/components/screens/GenreScreen.vue'))
const HomeScreen = defineAsyncComponent(() => import('@/components/screens/HomeScreen.vue'))
const NotFoundScreen = defineAsyncComponent(() => import('@/components/screens/NotFoundScreen.vue'))
const PlaylistScreen = defineAsyncComponent(() => import('@/components/screens/PlaylistScreen.vue'))
// QueueScreen and OfflineSongsScreen must NOT be lazy-loaded, so they work offline.
import QueueScreen from '@/components/screens/QueueScreen.vue'
import OfflineSongsScreen from '@/components/screens/OfflineSongsScreen.vue'
const RecentlyPlayedScreen = defineAsyncComponent(() => import('@/components/screens/RecentlyPlayedScreen.vue'))
const SearchExcerptsScreen = defineAsyncComponent(() => import('@/components/screens/search/SearchExcerptsScreen.vue'))
const SearchSongResultsScreen = defineAsyncComponent(
  () => import('@/components/screens/search/SearchPlayableResultsScreen.vue'),
)
const SettingsScreen = defineAsyncComponent(() => import('@/components/screens/SettingsScreen.vue'))
const HuntScreen = defineAsyncComponent(() => import('@/components/screens/hunting/HuntScreen.vue'))
const WatchesScreen = defineAsyncComponent(() => import('@/components/screens/hunting/WatchesScreen.vue'))
const JobsScreen = defineAsyncComponent(() => import('@/components/screens/hunting/JobsScreen.vue'))
const OrphansScreen = defineAsyncComponent(() => import('@/components/screens/hunting/OrphansScreen.vue'))
const UploadScreen = defineAsyncComponent(() => import('@/components/screens/UploadScreen.vue'))

const addedScreens = applyFilters<Partial<Record<ScreenName, Component>>>(Filter.SCREENS, {})

const { isMobile } = useViewport()
const { onRouteChanged, getCurrentScreen } = useRouter()

const screen = ref<ScreenName>('Home')
const loadedScreens = reactive<ScreenName[]>([])

onRouteChanged(route => {
  if (!loadedScreens.includes(route.screen)) {
    loadedScreens.push(route.screen)
  }

  screen.value = route.screen
})

const screenLoaded = (screenName: ScreenName) => loadedScreens.includes(screenName)

onMounted(() => {
  screen.value = getCurrentScreen()
  loadedScreens.push(screen.value)
})
</script>

<style scoped>
.main-content {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 0%;
  min-width: 0;
  margin: 12px 12px 0 0;
  overflow: hidden;
  border-radius: 28px;
  background: var(--schemes-surface);
}

.screens {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

@media (max-width: 768px) {
  .main-content {
    margin: 0;
    border-radius: 0;
  }
}

@media (max-width: 640px) and (min-width: 769px) {
  .main-content {
    margin-left: 12px;
  }
}
</style>
