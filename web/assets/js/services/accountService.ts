/**
 * The signed-in user's account: profile, password, API keys and alerts.
 */
import { http } from '@/services/http'

export type Role = 'admin' | 'user'
export type AccountStatus = 'pending' | 'unverified' | 'active' | 'disabled'

export interface Account {
  id: number
  username: string
  email: string | null
  email_verified: boolean
  role: Role
  status: AccountStatus
  password_change_required: boolean
  created_at: string
  /** Only for the account's owner: whether this server sends email. */
  mail_ready?: boolean
}

export interface ApiKeyInfo {
  id: number
  name: string
  created_at: string
  last_used_at: string | null
  current: boolean
}

export type AlertKind = 'youtube_music_expired' | 'watch_failing'

export interface AlertPreferences {
  /** Whether email reaches the user: the server sends mail, and their address is confirmed. */
  deliverable: boolean
  alerts: Record<AlertKind, boolean>
}

export interface LinkedAccount {
  id: number
  /** The provider's name, as the admin set it up. */
  provider: string
  email: string | null
  linked_at: string
  last_login_at: string | null
}

export const accountService = {
  me: () => http.get<Account>('me'),
  updateProfile: (profile: { username: string; email: string }) => http.put<Account>('me', profile),
  changePassword: (password: string, currentPassword = '') =>
    http.put('me/password', { current_password: currentPassword, password }),
  resendVerification: () => http.post('me/email/resend', {}),

  identities: () => http.get<LinkedAccount[]>('me/identities'),
  /** Where to send the browser to link an account at the provider. */
  linkIdentity: async () => (await http.post<{ url: string }>('me/identities/oidc', {})).url,
  unlinkIdentity: (id: number) => http.delete(`me/identities/${id}`),

  alerts: () => http.get<AlertPreferences>('me/alerts'),
  setAlerts: (changes: Partial<Record<AlertKind, boolean>>) => http.put<AlertPreferences>('me/alerts', changes),

  keys: () => http.get<ApiKeyInfo[]>('me/keys'),
  createKey: (name: string) => http.post<{ id: number; name: string; key: string }>('me/keys', { name }),
  revokeKey: (id: number) => http.delete(`me/keys/${id}`),
}
