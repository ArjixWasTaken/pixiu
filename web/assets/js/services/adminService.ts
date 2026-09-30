/**
 * Managing users, for admins.
 */
import { http } from '@/services/http'
import type { Account, Role } from '@/services/accountService'

export interface ManagedAccount extends Account {
  last_seen: string | null
  songs: number
  bytes: number
  /** What deleting the account frees: files no other library plays. */
  exclusive_bytes: number
  youtube_music: 'none' | 'valid' | 'degraded' | 'expired'
}

export interface StoreUsage {
  files: number
  bytes: number
  shared_files: number
}

export const adminService = {
  users: () => http.get<ManagedAccount[]>('admin/users'),
  createUser: (account: { username: string; email: string; password: string; role: Role }) =>
    http.post<Account>('admin/users', account),
  updateUser: (
    id: number,
    changes: Partial<{ role: Role; status: 'active' | 'disabled'; username: string; email: string }>,
  ) => http.patch<Account>(`admin/users/${id}`, changes),
  setTemporaryPassword: (id: number, password: string) => http.post(`admin/users/${id}/password`, { password }),
  sendPasswordReset: (id: number) => http.post(`admin/users/${id}/password-reset`, {}),
  resendVerification: (id: number) => http.post(`admin/users/${id}/verification`, {}),
  approveRegistration: (id: number) => http.post<Account>(`admin/registrations/${id}/approve`, {}),
  denyRegistration: (id: number) => http.post(`admin/registrations/${id}/deny`, {}),
  deleteUser: (id: number) => http.delete<{ freed_bytes: number }>(`admin/users/${id}`),
  storage: () => http.get<StoreUsage>('admin/storage'),
}
