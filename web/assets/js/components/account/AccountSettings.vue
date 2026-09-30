<template>
  <div class="flex flex-col gap-6" data-testid="account-settings">
    <ProfileGroup />
    <PasswordGroup />
    <AlertsGroup />
    <ApiKeysGroup />
  </div>
</template>

<script lang="ts" setup>
import AlertsGroup from '@/components/account/AlertsGroup.vue'
import ApiKeysGroup from '@/components/account/ApiKeysGroup.vue'
import PasswordGroup from '@/components/account/PasswordGroup.vue'
import ProfileGroup from '@/components/account/ProfileGroup.vue'
</script>
