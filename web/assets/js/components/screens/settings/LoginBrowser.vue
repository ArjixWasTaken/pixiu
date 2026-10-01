<template>
  <section class="flex flex-col gap-4">
    <header class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h3 class="text-lg font-semibold">Sign in inside the login browser</h3>
        <p class="text-(--schemes-on-surface-variant) max-w-[64ch]">
          A real browser runs on your server. Its screen streams here and your clicks and typing go to it, so Google’s
          normal sign-in works, two-factor included.
        </p>
      </div>
      <span :class="{ signed: status.logged_in }" class="pill">
        <span class="dot" />
        {{ statusLabel }}
      </span>
    </header>

    <div class="overflow-hidden rounded-xl border border-(--schemes-outline-variant) bg-black">
      <p
        class="flex items-center gap-2 px-3 py-1.5 border-b border-(--schemes-outline-variant) font-mono text-xs text-(--schemes-on-surface-variant)"
      >
        live · {{ status.host ?? (status.open ? 'loading…' : 'closed') }}
      </p>
      <canvas
        ref="canvas"
        class="block h-auto w-full outline-none focus:ring-2 focus:ring-(--schemes-primary)/60 focus:ring-inset"
        height="800"
        tabindex="0"
        width="1280"
      />
    </div>

    <!-- Owns the sign-in fields mirrored into the canvas, so password managers see a login form. -->
    <form id="login-mirror" ref="mirrorForm" action="#" />

    <p class="text-sm text-(--schemes-on-surface-variant)">
      <template v-if="mirrors">
        Your password manager can fill the sign-in fields. It sees them on {{ origin }}, so add that address to your
        Google login.
      </template>
      <template v-else>
        Want your password manager to fill this in? In Chrome, enable chrome://flags/#canvas-draw-element and reload.
        The sign-in fields then get real inputs laid over them. Typing works fine without it.
      </template>
    </p>

    <footer class="flex justify-end gap-2">
      <M3Button variant="text" @click.prevent="cancel">Cancel</M3Button>
      <M3Button :disabled="!status.logged_in" @click.prevent="finish">Done</M3Button>
    </footer>
  </section>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'
import { authService } from '@/services/authService'
import { huntingService } from '@/services/huntingService'
import { startLoginScreen, supportsMirrors } from '@/utils/loginScreen'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'

const emit = defineEmits<{ (e: 'done'): void; (e: 'cancel'): void }>()

const { handleHttpError } = useErrorHandler('dialog')

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const mirrorForm = useTemplateRef<HTMLFormElement>('mirrorForm')
const mirrors = supportsMirrors()
const origin = window.location.origin

// Asked every two seconds while the browser shows, so "Done" turns on once it holds a login.
// A failed ask is tried again at the next.
const polling = ref(true)

const { data } = useQuery({
  queryKey: ['hunting', 'login-status'],
  queryFn: () => huntingService.loginStatus(),
  refetchInterval: 2000,
  staleTime: 0,
  enabled: polling,
})

const status = computed(() => data.value ?? { open: true, logged_in: false, host: null })

const statusLabel = computed(() => {
  if (status.value.logged_in) {
    return 'Signed in: press Done'
  }

  return status.value.open ? 'Not signed in yet' : 'The login browser closed'
})

let stopScreen: (() => void) | null = null

const stop = () => {
  polling.value = false
  stopScreen?.()
  stopScreen = null
}

const finish = async () => {
  try {
    await huntingService.finishLogin()
    stop()
    emit('done')
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const cancel = async () => {
  stop()

  try {
    await huntingService.cancelLogin()
  } finally {
    emit('cancel')
  }
}

onMounted(() => {
  const scheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const key = encodeURIComponent(authService.getApiToken() ?? '')
  const url = `${scheme}//${window.location.host}${window.KOEL.base_url}api/sources/login/ws?api_key=${key}`

  stopScreen = startLoginScreen(canvas.value!, mirrorForm.value!, url)
})

onBeforeUnmount(stop)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.pill {
  @apply flex items-center gap-2 h-9 px-4 rounded-full bg-(--schemes-surface-container-high) text-sm;

  .dot {
    @apply size-2.5 rounded-full bg-(--schemes-tertiary) animate-pulse;
  }

  &.signed .dot {
    @apply bg-(--schemes-tertiary) animate-none;
  }
}
</style>
