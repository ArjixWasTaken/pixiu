import type { RenderOptions } from '@testing-library/vue'
import { cleanup, createEvent, fireEvent, render } from '@testing-library/vue'
import userEvent from '@testing-library/user-event'
import type { UserEvent } from '@testing-library/user-event'
import { afterEach, beforeEach, vi } from 'vite-plus/test'
import { defineComponent, h, nextTick, shallowRef } from 'vue'
import type { Pinia } from 'pinia'
import { createPinia, setActivePinia } from 'pinia'
import { DropdownMenuContent, DropdownMenuRoot } from 'reka-ui'
import factory from '@/__tests__/factory'
import { DialogBoxStub, MessageToasterStub, OverlayStub } from '@/__tests__/stubs'
import { useCommonStore } from '@/stores/commonStore'
import { useUserStore } from '@/stores/userStore'
import { http } from '@/services/http'
import type { Router } from 'vue-router'
import { createMemoryHistory } from 'vue-router'
import { ContextMenuKey, DialogBoxKey, MessageToasterKey, ModalKey, OverlayKey } from '@/config/symbols'
import { createAppRouter, resetRouting } from '@/router'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { noop } from '@/utils/helpers'
import { deepMerge, setPropIfNotExists } from '@/__tests__/utils'
import { eventBus } from '@/utils/eventBus'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { queryClient } from '@/services/queryClient'
import { setViewport } from '@/composables/useViewport'

// Specs see a phone-sized, wide window with a mouse unless they say otherwise.
setViewport({ mobile: true, wide: true })

// A request that fails in a spec fails at once.
queryClient.setDefaultOptions({
  ...queryClient.getDefaultOptions(),
  queries: { ...queryClient.getDefaultOptions().queries, retry: false },
})

class TestHarness {
  /** A fresh one for each spec, in memory, starting at Home. */
  public router!: Router
  /** The stores, fresh for each spec. */
  public pinia!: Pinia
  public user: UserEvent
  private backupMethods = new Map()
  private realFetch = globalThis.fetch

  public constructor() {
    // One from the start, for specs that render before the hooks run; each spec gets its own.
    this.router = createAppRouter(createMemoryHistory())
    this.pinia = createPinia()
    setActivePinia(this.pinia)
    this.user = userEvent.setup({ delay: null }) // @see https://github.com/testing-library/user-event/issues/833

    this.setReadOnlyProperty(navigator, 'clipboard', {
      writeText: vi.fn(),
    })
  }

  /** Whether each spec starts signed in (as a user, not an admin). */
  public signedInByDefault = true

  public beforeEach(cb?: Closure) {
    beforeEach(async () => {
      this.pinia = createPinia()
      setActivePinia(this.pinia)
      this.signedInByDefault && this.actingAsUser()
      this.router = createAppRouter(createMemoryHistory())
      await this.router.push('/home')

      this.mock(http, 'request').mockResolvedValue({}) // prevent actual HTTP requests from being made
      // The Subsonic client uses fetch: answer every call with an empty success. Kept out of the
      // mock registry, so specs that swap fetch themselves and call restoreAllMocks() keep theirs.
      this.realFetch = globalThis.fetch
      globalThis.fetch = vi.fn(async () => new Response(JSON.stringify({ 'subsonic-response': { status: 'ok' } })))

      useCommonStore().state.song_length = 10
      useCommonStore().state.allows_download = true
      useCommonStore().state.supports_batch_downloading = true
      useCommonStore().state.supports_transcoding = true

      this.setDefaultBranding()
      cb?.()
    })
  }

  public afterEach(cb?: Closure) {
    afterEach(() => {
      // Unmounted first: what a component teleported (menus, dialogs) goes with it.
      cleanup()
      document.body.innerHTML = ''
      setViewport({ mobile: true, wide: true })
      localStorage.clear()
      queryClient.clear()
      this.restoreAllMocks()
      globalThis.fetch = this.realFetch
      eventBus.all.clear()
      resetRouting()
      cb?.()
    })
  }

  private setDefaultBranding() {
    window.KOEL.branding = {
      name: 'Koel',
      logo: '',
      cover: '',
    }
  }

  public readonly auth = (user?: CurrentUser) => this.actingAsUser(user)

  public actingAsUser(user?: CurrentUser) {
    useUserStore().state.current = user || (factory('user').state('current').make() as CurrentUser)
    usePreferenceStore().init(useUserStore().state.current.preferences)
    return this
  }

  public actingAsAdmin() {
    return this.actingAsUser(factory('user').state('admin').make() as CurrentUser)
  }

  public mock<T>(obj: T, methodName: MethodOf<Required<T>>, implementation?: any) {
    // check if the method is already mocked, and if so, use it instead of creating a new mock
    for (const [key, _] of this.backupMethods.entries()) {
      if (key[0] !== obj || key[1] !== methodName) {
        continue
      }

      const existingMock = obj[methodName] as unknown as ReturnType<typeof vi.fn>

      if (implementation !== undefined) {
        existingMock.mockImplementation(implementation instanceof Function ? implementation : () => implementation)
      }

      return existingMock
    }

    const mock = vi.fn()

    if (implementation !== undefined) {
      mock.mockImplementation(implementation instanceof Function ? implementation : () => implementation)
    }

    this.backupMethods.set([obj, methodName], obj[methodName])

    // @ts-ignore
    obj[methodName] = mock

    return mock
  }

  public restoreAllMocks() {
    this.backupMethods.forEach((fn, [obj, methodName]) => (obj[methodName] = fn))
    this.backupMethods.clear()

    return this
  }

  public render(component: any, options: RenderOptions = {}) {
    return render(
      component,
      deepMerge(
        {
          global: {
            directives: {
              'koel-focus': {},
              'koel-hide-broken-icon': {},
              'koel-overflow-fade': {},
              'koel-new-tab': {},
            },
          },
        },
        this.supplyRequiredProvides(options),
      ),
    )
  }

  public async withCustomBranding(branding: Branding, cb: Closure) {
    window.KOEL.branding = branding
    await cb()
    this.setDefaultBranding()

    return this
  }

  public async withDemoMode(cb: Closure) {
    window.KOEL.is_demo = true
    await cb()
    window.KOEL.is_demo = false

    return this
  }

  public stub(testId = 'stub', asModelComponent = false, defaultValue?: any) {
    if (!asModelComponent) {
      return defineComponent({
        template: `<br data-testid="${testId}"/>`,
      })
    }

    return defineComponent({
      template: `<input data-testid="${testId}" @input="$emit('update:modelValue', $event.target.value)" />`,
      emits: ['update:modelValue'],
      mounted() {
        defaultValue && this.$emit('update:modelValue', defaultValue)
      },
    })
  }

  public async tick(count = 1) {
    for (let i = 0; i < count; ++i) {
      await nextTick()
    }
  }

  public setReadOnlyProperty<T>(obj: T, prop: keyof T, value: any) {
    return Object.defineProperties(obj, {
      [prop]: {
        value,
        configurable: true,
        writable: true,
      },
    })
  }

  public async type(element: HTMLElement, value: string) {
    await this.user.clear(element)
    await this.user.type(element, value)
  }

  public async trigger(element: HTMLElement, key: string, options: object = {}) {
    await fireEvent(element, createEvent[key](element, options))
  }

  private supplyRequiredProvides(options: RenderOptions) {
    options.global = options.global || {}
    options.global.provide = options.global.provide || {}
    // RouterView and the like find the spec's router; components, its stores.
    options.global.plugins = [
      ...(options.global.plugins ?? []),
      this.router,
      this.pinia,
      [VueQueryPlugin, { queryClient }],
    ]

    setPropIfNotExists(options.global.provide, DialogBoxKey, DialogBoxStub)
    setPropIfNotExists(options.global.provide, MessageToasterKey, MessageToasterStub)
    setPropIfNotExists(options.global.provide, OverlayKey, OverlayStub)

    setPropIfNotExists(
      options.global.provide,
      ContextMenuKey,
      shallowRef({
        component: null,
        position: { top: 0, left: 0 },
      }),
    )

    setPropIfNotExists(
      options.global.provide,
      ModalKey,
      shallowRef({
        component: null,
      }),
    )

    return options
  }

  public createAudioPlayer() {
    if (document.querySelector('#audio-player')) {
      return
    }

    document.body.innerHTML = '<audio id="audio-player" crossorigin="anonymous"/>'

    window.AudioContext = vi.fn().mockImplementation(() => ({
      createMediaElementSource: vi.fn(noop),
    }))
  }

  /**
   * Renders a context menu's items as they appear: inside an open menu
   * (Reka UI's items need one around them).
   */
  public renderMenu(component: any, options: RenderOptions = {}) {
    const props = (options.props ?? {}) as Record<string, unknown>
    const anchor = { getBoundingClientRect: () => DOMRect.fromRect({ x: 0, y: 0, width: 0, height: 0 }) }

    const Menu = defineComponent({
      setup: () => () =>
        h(DropdownMenuRoot, { open: true, modal: false }, () =>
          h(DropdownMenuContent, { reference: anchor, class: 'context-menu' }, () => h(component, props)),
        ),
    })

    return this.render(Menu, { ...options, props: {} })
  }

  /** Goes to a path, as the app would. */
  public async visit(path: string) {
    await this.router.push(path.startsWith('/') ? path : `/${path}`)
    return this
  }

  public readonly factory = factory
}

export function createHarness(overrides?: {
  beforeEach?: () => void
  afterEach?: () => void
  authenticated?: boolean
}) {
  const h = new TestHarness()
  h.signedInByDefault = overrides?.authenticated ?? true

  // Signed in from the start too, for what specs do before their hooks run.
  if (h.signedInByDefault) {
    h.actingAsUser()
  }

  h.beforeEach(overrides?.beforeEach)
  h.afterEach(overrides?.afterEach)

  return h
}
