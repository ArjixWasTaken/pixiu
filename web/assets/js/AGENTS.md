# Frontend Conventions

## Components and icons

- Build controls from `components/m3`: `M3Button`, `M3IconButton`, `M3TextField`, `M3Select`, `M3Switch`, `M3Tabs`. There is no second kit.
- Icons are Material Symbols through `<M3Icon name="…" />` (`fill` for the filled style). Don't add icon packages.
- Colors are the Material 3 roles, as `--schemes-*` variables (e.g. `text-(--schemes-on-surface-variant)`). Don't hard-code colors.
- UI text is sentence case ("Add to queue", "New smart playlist"), and an action keeps one name across buttons, menus and toasts.
- Menus, popovers, dialogs, toasts and tabs are Reka UI primitives inside our components: `useContextMenu` (its items: `MenuItem`, `Separator`), `M3MenuPopover` + `M3MenuItem`, `Popover`, `useModal`, `useDialogBox`, `useMessageToaster`, `M3Tabs`. Use those; don't hand-roll focus, keyboard or click-outside handling. Their contents render in a portal, so style them through what they take (`min-width`, a class on the panel), not `:deep()` from the caller.
- Long lists go through `VirtualScroller` (rows measured; `itemHeight` is an estimate) and `VirtualGridScroller` (cards), on TanStack Virtual. Both say `scrolled-to-end` near their end.

## Routing

- Routes are vue-router records in `config/routes.ts`, with `meta.screen` (and `layout`, `public`, `guard`). Screens use the `useRouter()` facade from `@/composables/useRouter`: `url(name, params)`, `go`, `getRouteParam`, `onScreenActivated`, `onRouteChanged`. Links are plain `<a :href="url(…)">`; in-app ones are routed without a reload.
- What a page shows within itself (its tab) lives in the URL's hash, through `useHash`/`useHashTab` from `@/composables/useHash` (`/settings#admin-users`), so a reload or a link opens it. Setting it replaces the URL; it never goes through the router.

## State and Server Data

- Stores are Pinia setup stores: `useXStore()`. A component calls it once at the top of its setup (`const albumStore = useAlbumStore()`); other code calls it where it needs it, never at a module's top level (no Pinia is active yet there).
- Entity stores (songs, albums, artists, users) keep a vault: one reactive object per entity, shared by every list. Server data goes through the store's `syncWithVault` before it's shown.
- What comes from the server is kept by TanStack Query (`queryClient` from `@/services/queryClient`): `useQuery` in components, `queryClient.fetchQuery` in stores, and `invalidateQueries` after a write. Keys nest from the entity down (`['album', id, 'songs']`), so invalidating the album covers all it holds. Paged lists use `useListPages`.
- The service worker is `service-worker.ts` (vite-plugin-pwa, Workbox): it precaches the player, keeps the start-up requests, and plays the songs made available offline (`useOfflinePlayback`).

## TypeScript Conventions

- Always prefer generics over type casting when the API supports it (e.g. `container.querySelector<HTMLElement>('.foo')` instead of `container.querySelector('.foo') as HTMLElement`).
- Do not add explicit return types when they can be inferred by the compiler. Only annotate return types when inference is insufficient or ambiguous.
- When using `setTimeout`, `setInterval`, or `requestAnimationFrame`, always ensure they are cleaned up: on component unmount (`onBeforeUnmount`), on state transitions that invalidate them (e.g. drop cancels a pending expand), and when the operation completes. Treat every timer/rAF as a resource that must be explicitly released.

## Vue Template Conventions

- Always use Vue's same-name shorthand for bindings: `:foo` instead of `:foo="foo"`. This applies to props, components, and any v-bind where the attribute name matches the variable name.

## Vue Forms

- Any Vue surface that takes user input and commits it on submit must use the `useForm` composable from `@/composables/useForm` — including inline composers, popovers, and mini name-prompts that aren't named `*Form.vue`. Don't roll your own `ref<string>('')` + manual submit handling.
- Pair it with the canonical wiring: `<form @submit.prevent="handleSubmit" @keydown.esc="maybeClose">`, inputs use `v-koel-focus` (not manual `onMounted` focus) and `required` (not manual `:disabled`), Save is `<M3Button type="submit">`, Cancel is `<M3Button type="button" variant="text" @click.prevent="maybeClose">` before it, and `maybeClose` does `if (isPristine() || (await showConfirmDialog(...))) emit('cancel')`.
- For purely-local submits (no server call), pass `useOverlay: false` and have `onSubmit` just emit. Use the optional `validator` callback for non-HTML5 rules (e.g. trim/whitespace).
- Read `web/assets/js/components/playlist/CreatePlaylistFolderForm.vue` before writing a new form — that's the reference shape.

## Vue Component Decomposition

- Always try to break Vue components into smaller, self-managed-state subcomponents. A component that hosts multiple stages, multiple modes, or multiple distinct UI shapes should split each into its own focused child. The parent becomes a thin orchestrator (state machine + API calls + composition); each child owns one shape with clear props in and events out, no service dependencies of its own, and is testable in isolation with minimal mocks. Reference shape: `screens/settings/admin/UsersSettings.vue` (orchestrator: the API calls and confirmations) → `AccountRow.vue` (one account: props in, events out).

## Vue Component Styling

- Put shared/base Tailwind classes directly on the HTML element via the `class` attribute.
- For variant-specific styles (e.g. modes, states), use custom CSS classes (`.initial`, `.chat`, `.user`, `.error`, etc.) with `@apply` in a scoped `<style>` block.
- Do NOT build class strings in JavaScript arrays or computed properties.

## Frontend Testing

- Each spec gets a fresh Pinia and router, starts signed in, and has the query cache and localStorage emptied after it (`createHarness`). Mock a store's action on its instance: `h.mock(useAlbumStore(), 'paginate')`.
- A context menu's component renders inside an open menu with `h.renderMenu(Component, { props })`; its submenus open on a click on their item.
- jsdom lays nothing out: specs of virtual lists give elements the sizes a browser would (see `VirtualScroller.spec.ts`).
- Prefer semantic queries (`getByRole`, `getByLabelText`, `getByText`) via `screen` from `@testing-library/vue`. Use `data-testid` only as a last resort when no semantic query is available.
- `getBy*` queries already throw if the element is not found, so never wrap them in `expect().toBeTruthy()`. Just call `screen.getByTestId('foo')` directly — the throw is the assertion. Use `expect(screen.queryBy*()).toBeNull()` to assert absence.
