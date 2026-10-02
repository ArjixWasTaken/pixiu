<template>
  <div class="top-bar">
    <M3IconButton v-if="isMobile" icon="menu" label="Open navigation" @click="openDrawer" />
    <!-- Discover has its own search, of a platform: one field is enough. -->
    <SearchForm v-if="!onDiscover" class="search">
      <template v-if="isMobile" #trailing>
        <ProfileDropdown :size="30" />
      </template>
    </SearchForm>
    <span class="spacer" />
    <ProfileDropdown v-if="!isMobile || onDiscover" :size="isMobile ? 30 : 40" />
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ProfileDropdown from '@/components/layout/main-wrapper/side-sheet/ProfileDropdown.vue'
import SearchForm from '@/components/ui/SearchForm.vue'

const { isMobile } = useViewport()
const { isCurrentScreen } = useRouter()

const onDiscover = computed(() => isCurrentScreen('Hunt'))

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

@media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
  .top-bar {
    gap: 4px;
    padding: 4px 8px 4px 4px;
  }

  .search {
    max-width: none;
  }

  /* Beside the search field, nothing to fill; without it (Discover), the avatar still goes right. */
  .search + .spacer {
    display: none;
  }
}
</style>
