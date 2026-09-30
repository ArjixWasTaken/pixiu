import { http } from '@/services/http'
import { useLocalStorage } from '@/composables/useLocalStorage'
import { use } from '@/utils/helpers'

const API_TOKEN_STORAGE_KEY = 'api-token'
const AUDIO_TOKEN_STORAGE_KEY = 'audio-token'
const REDIRECT_KEY = 'redirect'

const { get: lsGet, set: lsSet, remove: lsRemove } = useLocalStorage(false) // authentication local storage data aren't namespaced

export interface AuthStatus {
  /** Whether píxiū has its admin yet; until then, the login screen creates it. */
  claimed: boolean
  /** Whether a forgotten password can be reset by email. */
  password_reset: boolean
}

export const authService = {
  status: () => http.get<AuthStatus>('auth/status'),

  /** Emails a reset link to the account, if there is one. Says nothing either way. */
  forgot: (login: string) => http.post('auth/forgot', { login }),

  /** Follows a reset link: sets the new password and signs in with it. */
  async resetPassword(token: string, password: string) {
    this.setTokensUsingCompositeToken(await http.post<CompositeToken>('auth/reset', { token, password }))
  },

  /** Follows a link confirming an email address. */
  verifyEmail: (token: string) => http.post('auth/verify-email', { token }),

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
