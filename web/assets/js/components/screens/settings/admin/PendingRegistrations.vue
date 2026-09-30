<template>
  <section class="flex flex-col gap-3" data-testid="pending-registrations">
    <header>
      <h3 class="m3-title-large">Asking for an account</h3>
      <p class="m3-body-medium text-(--schemes-on-surface-variant)">
        Approved, they get an email to confirm their address, then sign in. Denied, they get a short note.
      </p>
    </header>
    <M3Card v-for="request in requests" :key="request.id" class="request" data-testid="registration" variant="outlined">
      <M3Avatar :name="request.username" :size="40" />
      <div class="flex-1 min-w-0">
        <p class="m3-title-medium truncate">{{ request.username }}</p>
        <p class="m3-body-medium text-(--schemes-on-surface-variant) truncate">
          {{ request.email }} · asked {{ timeAgo(request.created_at) }}
        </p>
      </div>
      <div class="flex gap-2">
        <M3Button variant="outlined" @click="$emit('deny', request)">Deny</M3Button>
        <M3Button @click="$emit('approve', request)">Approve</M3Button>
      </div>
    </M3Card>
  </section>
</template>

<script lang="ts" setup>
import type { ManagedAccount } from '@/services/adminService'
import { timeAgo } from '@/utils/formatters'

import M3Avatar from '@/components/m3/M3Avatar.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'

defineProps<{ requests: ManagedAccount[] }>()
defineEmits<{ (e: 'approve', request: ManagedAccount): void; (e: 'deny', request: ManagedAccount): void }>()
</script>

<style scoped>
.request {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px;
  padding: 16px;
}
</style>
