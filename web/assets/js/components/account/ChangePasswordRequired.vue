<template>
  <div class="page" data-testid="change-password-required">
    <div class="w-full max-w-[480px] flex flex-col gap-4">
      <h1 class="m3-headline-small text-(--schemes-on-surface)">Choose your password</h1>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        You signed in with a temporary password. Pick one of your own to carry on.
      </p>
      <PasswordGroup temporary @changed="done" />
      <div class="flex justify-end">
        <M3Button variant="text" @click="signOut">Sign out</M3Button>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { eventBus } from '@/utils/eventBus'
import { useUserStore } from '@/stores/userStore'
import M3Button from '@/components/m3/M3Button.vue'
import PasswordGroup from '@/components/account/PasswordGroup.vue'

const userStore = useUserStore()

const done = () => {
  if (userStore.state.current) {
    userStore.state.current.password_change_required = false
  }
}

const signOut = () => eventBus.emit('LOG_OUT')
</script>

<style scoped>
.page {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100dvh;
  padding: 24px 16px;
  background: var(--schemes-surface-container);
}
</style>
