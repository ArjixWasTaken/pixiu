<template>
  <section class="flex flex-col gap-4">
    <header class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h3 class="text-lg font-semibold">Sign in inside the login browser</h3>
        <p class="text-k-fg-70 max-w-[64ch]">
          A real browser runs on your server. Its screen streams here and your clicks and typing go to it, so Google’s
          normal sign-in works, two-factor included.
        </p>
      </div>
      <span :class="{ signed: status.logged_in }" class="pill">
        <span class="dot" />
        {{ statusLabel }}
      </span>
    </header>

    <div class="overflow-hidden rounded-xl border border-k-fg-10 bg-black">
      <p class="flex items-center gap-2 px-3 py-1.5 border-b border-k-fg-10 font-mono text-xs text-k-fg-70">
        live · {{ status.host ?? (status.open ? 'loading…' : 'closed') }}
      </p>
      <canvas
        ref="canvas"
        class="block h-auto w-full outline-none focus:ring-2 focus:ring-k-highlight/60 focus:ring-inset"
        height="800"
        tabindex="0"
        width="1280"
      />
    </div>

    <!-- Owns the sign-in fields mirrored into the canvas, so password managers see a login form. -->
    <form id="login-mirror" ref="mirrorForm" action="#" />

    <p class="text-sm text-k-fg-70">
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
      <Btn variant="ghost" @click.prevent="cancel">Cancel</Btn>
      <Btn :disabled="!status.logged_in" @click.prevent="finish">Done</Btn>
    </footer>
  </section>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, reactive, useTemplateRef } from 'vue'
import { authService } from '@/services/authService'
import { huntingService } from '@/services/huntingService'
import { startLoginScreen, supportsMirrors } from '@/utils/loginScreen'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'

const emit = defineEmits<{ (e: 'done'): void; (e: 'cancel'): void }>()

const { handleHttpError } = useErrorHandler('dialog')

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const mirrorForm = useTemplateRef<HTMLFormElement>('mirrorForm')
const mirrors = supportsMirrors()
const origin = window.location.origin

const status = reactive({ open: true, logged_in: false, host: null as string | null })

const statusLabel = computed(() => {
  if (status.logged_in) {
    return 'Signed in: press Done'
  }

  return status.open ? 'Not signed in yet' : 'The login browser closed'
})

let stopScreen: (() => void) | null = null
let pollTimer: number | undefined

// Enable "Done" once the browser holds a login.
const poll = async () => {
  try {
    Object.assign(status, await huntingService.loginStatus())
  } catch {
    // The next poll tries again.
  }
}

const stop = () => {
  window.clearInterval(pollTimer)
  pollTimer = undefined
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
  pollTimer = window.setInterval(poll, 2000)
  poll()
})

onBeforeUnmount(stop)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.pill {
  @apply flex items-center gap-2 h-9 px-4 rounded-full bg-k-fg-5 text-sm;

  .dot {
    @apply size-2.5 rounded-full bg-k-warning animate-pulse;
  }

  &.signed .dot {
    @apply bg-k-success animate-none;
  }
}
</style>
