/**
 * The signed-in user's account: profile, password and API keys.
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
}

export interface ApiKeyInfo {
  id: number
  name: string
  created_at: string
  last_used_at: string | null
  current: boolean
}

export const accountService = {
  me: () => http.get<Account>('me'),
  updateProfile: (profile: { username: string; email: string }) => http.put<Account>('me', profile),
  changePassword: (password: string, currentPassword = '') =>
    http.put('me/password', { current_password: currentPassword, password }),

  keys: () => http.get<ApiKeyInfo[]>('me/keys'),
  createKey: (name: string) => http.post<{ id: number; name: string; key: string }>('me/keys', { name }),
  revokeKey: (id: number) => http.delete(`me/keys/${id}`),
}
