<template>
  <div class="main-wrapper">
    <SideBar />
    <MainContent />
    <ModalWrapper />
  </div>
</template>

<script lang="ts" setup>
import MainContent from '@/components/layout/main-wrapper/MainContent.vue'
import SideBar from '@/components/layout/main-wrapper/sidebar/Sidebar.vue'
import ModalWrapper from '@/components/layout/ModalWrapper.vue'
import { useCoverTheme } from '@/composables/useCoverTheme'

useCoverTheme()
</script>

<style scoped>
.main-wrapper {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>
