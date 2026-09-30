import { http } from '@/services/http'
import { useLocalStorage } from '@/composables/useLocalStorage'
import { use } from '@/utils/helpers'

const API_TOKEN_STORAGE_KEY = 'api-token'
const AUDIO_TOKEN_STORAGE_KEY = 'audio-token'
const REDIRECT_KEY = 'redirect'

const { get: lsGet, set: lsSet, remove: lsRemove } = useLocalStorage(false) // authentication local storage data aren't namespaced

export const authService = {
  /** Whether píxiū has its admin yet; until then, the login screen creates it. */
  claimed: async () => (await http.get<{ claimed: boolean }>('auth/status')).claimed,

  /** Creates the admin account of a fresh píxiū, and signs in as it. */
  async claim(username: string, password: string) {
    this.setTokensUsingCompositeToken(await http.post<CompositeToken>('auth/setup', { username, password }))
  },

  /** Signs in with a username or an email. */
  async login(login: string, password: string) {
    this.setTokensUsingCompositeToken(await http.post<CompositeToken>('auth/login', { username: login, password }))
    this.maybeRedirect()
  },

  async logout() {
    await http.delete('auth/session')
    this.destroy()
  },

  getApiToken: () => lsGet<string>(API_TOKEN_STORAGE_KEY),

  hasApiToken() {
    return Boolean(this.getApiToken())
  },

  setApiToken: (token: string) => lsSet(API_TOKEN_STORAGE_KEY, token),

  setTokensUsingCompositeToken(compositeToken: CompositeToken) {
    this.setApiToken(compositeToken.token)
    this.setAudioToken(compositeToken['audio-token'])
  },

  destroy: () => {
    lsRemove(API_TOKEN_STORAGE_KEY)
    lsRemove(AUDIO_TOKEN_STORAGE_KEY)
  },

  setAudioToken: (token: string) => lsSet(AUDIO_TOKEN_STORAGE_KEY, token),

  getAudioToken: () => {
    // for backward compatibility, we first try to get the audio token, and fall back to the (full-privileged) API token
    return lsGet(AUDIO_TOKEN_STORAGE_KEY) || lsGet(API_TOKEN_STORAGE_KEY)
  },

  setRedirect: (url?: string) => lsSet(REDIRECT_KEY, url || location.toString()),

  hasRedirect: () => Boolean(lsGet(REDIRECT_KEY)),

  maybeRedirect: () =>
    use(lsGet<string | null>(REDIRECT_KEY), url => {
      lsRemove(REDIRECT_KEY)
      location.assign(url)
    }),
}
