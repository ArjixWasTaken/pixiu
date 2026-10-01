import { http } from '@/services/http'
import { useAppStorage } from '@/composables/useUserStorage'
import { use } from '@/utils/helpers'

// Not kept per user: they say who the user is.
const apiToken = useAppStorage<string | null>('api-token', null)
const audioToken = useAppStorage<string | null>('audio-token', null)
const redirect = useAppStorage<string | null>('redirect', null)

export interface AuthStatus {
  /** Whether píxiū has its admin yet; until then, the login screen creates it. */
  claimed: boolean
  /** Whether a forgotten password can be reset by email. */
  password_reset: boolean
  /** Whether anyone may ask for an account. */
  registration: boolean
  /** The single sign-on provider people may sign in with. */
  sso: { name: string } | null
}

export const authService = {
  status: () => http.get<AuthStatus>('auth/status'),

  /** Asks for an account, which an admin approves or denies. */
  register: (account: { username: string; email: string; password: string }) => http.post('auth/register', account),

  /** Emails a reset link to the account, if there is one. Says nothing either way. */
  forgot: (login: string) => http.post('auth/forgot', { login }),

  /** Follows a reset link: sets the new password and signs in with it. */
  async resetPassword(token: string, password: string) {
    this.setTokensUsingCompositeToken(await http.post<CompositeToken>('auth/reset', { token, password }))
  },

  /** Where single sign-on starts: the browser goes there, and on to the provider. */
  ssoStartUrl: () => `${window.KOEL.base_url}api/auth/oidc/start`,

  /** Trades a single sign-on's one-time code for a token. */
  async exchangeSsoCode(code: string) {
    this.setTokensUsingCompositeToken(await http.post<CompositeToken>('auth/oidc/exchange', { code }))
  },

  /** Follows a link confirming an email address; says whether that opened the account. */
  verifyEmail: async (token: string) => (await http.post<{ opened: boolean }>('auth/verify-email', { token })).opened,

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

  getApiToken: () => apiToken.value,

  hasApiToken() {
    return Boolean(this.getApiToken())
  },

  setApiToken: (token: string) => (apiToken.value = token),

  setTokensUsingCompositeToken(compositeToken: CompositeToken) {
    this.setApiToken(compositeToken.token)
    this.setAudioToken(compositeToken['audio-token'])
  },

  destroy: () => {
    apiToken.value = null
    audioToken.value = null
  },

  setAudioToken: (token: string) => (audioToken.value = token),

  getAudioToken: () => {
    // for backward compatibility, we first try to get the audio token, and fall back to the (full-privileged) API token
    return audioToken.value || apiToken.value
  },

  setRedirect: (url?: string) => (redirect.value = url || location.toString()),

  hasRedirect: () => Boolean(redirect.value),

  maybeRedirect: () =>
    use(redirect.value, url => {
      redirect.value = null
      location.assign(url)
    }),
}
