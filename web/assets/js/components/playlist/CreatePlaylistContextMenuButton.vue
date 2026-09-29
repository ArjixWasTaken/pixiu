<template>
  <M3IconButton icon="add" label="Create a new playlist or folder" @click.stop.prevent="requestContextMenu" />
</template>

<script lang="ts" setup>
import M3IconButton from '@/components/m3/M3IconButton.vue'
import { useContextMenu } from '@/composables/useContextMenu'
import { defineAsyncComponent } from '@/utils/helpers'

const ContextMenu = defineAsyncComponent(() => import('@/components/playlist/CreatePlaylistContextMenu.vue'))

const { openContextMenu } = useContextMenu()

const requestContextMenu = (e: MouseEvent) => {
  const { bottom, right } = (e.currentTarget as HTMLButtonElement).getBoundingClientRect()

  openContextMenu(ContextMenu, {
    top: bottom,
    left: right,
  })
}
</script>
