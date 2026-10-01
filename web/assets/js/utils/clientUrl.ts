/** Where the app is served from: `/`, unless the page says otherwise. */
export const basePath = () => new URL(window.KOEL?.base_url ?? '/', location.origin).pathname

/** A path within the app, without the base it is served under (`/music/albums` → `/albums`). */
export const toClientPath = (path: string) => {
  const base = basePath()

  if (base !== '/' && path.startsWith(base)) {
    path = path.substring(base.length - 1)
  }

  return path.startsWith('/') ? path : `/${path}`
}
