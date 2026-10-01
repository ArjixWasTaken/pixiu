<template>
  <section id="mainContent" class="main-content" data-testid="main-content">
    <TopBar v-if="isMobile" />
    <NowPlayingPanel v-if="!isMobile" />
    <div class="screens">
      <NotFoundScreen v-if="notFound" />
      <!-- The 404 screen shows in place: the kept screens stay as they were. -->
      <div v-show="!notFound" class="screens">
        <RouterView v-slot="{ Component, route }">
          <KeepAlive :include="keptAlive">
            <component :is="Component" :key="route.meta.screen" />
          </KeepAlive>
        </RouterView>
      </div>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { RouterView } from 'vue-router'
import { notFound } from '@/router'
import { defineAsyncComponent } from '@/utils/helpers'
import { useViewport } from '@/composables/useViewport'

import TopBar from '@/components/layout/main-wrapper/TopBar.vue'
import NowPlayingPanel from '@/components/layout/now-playing/NowPlayingPanel.vue'

const NotFoundScreen = defineAsyncComponent(() => import('@/components/screens/NotFoundScreen.vue'))

/**
 * Screens with lists and state of their own (scroll position, a filter) stay
 * alive when left; the others start afresh each time.
 */
const keptAlive = [
  'HomeScreen',
  'QueueScreen',
  'AllSongsScreen',
  'AlbumListScreen',
  'ArtistListScreen',
  'PlaylistScreen',
  'FavoritesScreen',
  'RecentlyPlayedScreen',
  'OfflineSongsScreen',
  'UploadScreen',
  'SearchExcerptsScreen',
  'GenreScreen',
  'GenreListScreen',
  'HuntScreen',
]

const { isMobile } = useViewport()
</script>

<style scoped>
.main-content {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 0%;
  min-width: 0;
  overflow: hidden;
  background: var(--schemes-surface);
}

.screens {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
