<template>
  <div class="top-bar">
    <M3IconButton v-if="isMobile" icon="menu" label="Open navigation" @click="openDrawer" />
    <SearchForm class="search">
      <template v-if="isMobile" #trailing>
        <ProfileDropdown :size="30" />
      </template>
    </SearchForm>
    <span class="spacer" />
    <ProfileDropdown v-if="!isMobile" />
  </div>
</template>

<script lang="ts" setup>
import { eventBus } from '@/utils/eventBus'
import { useViewport } from '@/composables/useViewport'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ProfileDropdown from '@/components/layout/main-wrapper/side-sheet/ProfileDropdown.vue'
import SearchForm from '@/components/ui/SearchForm.vue'

const { isMobile } = useViewport()

const openDrawer = () => eventBus.emit('TOGGLE_SIDEBAR')
</script>

<style scoped>
.top-bar {
  position: relative;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px 8px 24px;
  flex-shrink: 0;
}

.search {
  flex: 1;
  min-width: 0;
  max-width: 560px;
}

.spacer {
  flex: 1;
}

@media (max-width: 768px) {
  .top-bar {
    gap: 4px;
    padding: 4px 8px 4px 4px;
  }

  .search {
    max-width: none;
  }

  .spacer {
    display: none;
  }
}
</style>
