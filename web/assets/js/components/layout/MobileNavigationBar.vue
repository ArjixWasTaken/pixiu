<template>
  <M3NavigationBar :items :value="current" data-testid="mobile-navigation" @select="go(url(routeOf($event)))" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import type { RouteName } from '@/config/routes'
import { useRouter } from '@/composables/useRouter'
import type { M3NavItem } from '@/components/m3/navigation'

import M3NavigationBar from '@/components/m3/M3NavigationBar.vue'

const { go, url, isCurrentScreen } = useRouter()

const destinations: Array<M3NavItem & { route: RouteName; screens: ScreenName[] }> = [
  { id: 'Home', label: 'Home', icon: 'home', route: 'home', screens: ['Home'] },
  { id: 'Songs', label: 'Songs', icon: 'music_note', route: 'songs.index', screens: ['Songs'] },
  { id: 'Albums', label: 'Albums', icon: 'album', route: 'albums.index', screens: ['Albums', 'Album'] },
  { id: 'Artists', label: 'Artists', icon: 'artist', route: 'artists.index', screens: ['Artists', 'Artist'] },
  { id: 'Hunt', label: 'Discover', icon: 'travel_explore', route: 'hunt', screens: ['Hunt'] },
]

const items = computed(() => destinations.map(item => ({ ...item, href: url(item.route) })))
const current = computed(() => destinations.find(item => isCurrentScreen(...item.screens))?.id ?? '')
const routeOf = (item: M3NavItem) => destinations.find(({ id }) => id === item.id)!.route
</script>
