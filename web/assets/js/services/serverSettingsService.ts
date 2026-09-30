/**
 * The server's settings, for admins: where people reach píxiū, and the mail server.
 */
import { http } from '@/services/http'

export type MailSecurity = 'none' | 'starttls' | 'tls'

export interface MailServer {
  host: string
  port: number
  security: MailSecurity
  username: string | null
  /** The password is never sent back; only whether there is one. */
  password_set: boolean
  from: string
}

export interface ServerSettings {
  public_url: string | null
  registration_open: boolean
  /** Whether email with links in it can go out: a mail server and the public address are set. */
  mail_ready: boolean
  smtp: MailServer | null
}

export interface MailServerForm {
  host: string
  port: number
  security: MailSecurity
  username: string
  /** Left out keeps the stored one; empty removes it. */
  password?: string
  from: string
}

export const serverSettingsService = {
  get: () => http.get<ServerSettings>('admin/settings'),
  setPublicUrl: (publicUrl: string | null) =>
    http.put<ServerSettings>('admin/settings/server', { public_url: publicUrl }),
  setMailServer: (form: MailServerForm) => http.put<ServerSettings>('admin/settings/smtp', form),
  removeMailServer: () => http.delete<ServerSettings>('admin/settings/smtp'),
  setRegistrationOpen: (open: boolean) => http.put<ServerSettings>('admin/settings/registration', { open }),
  /** Sends a test email now; fails with the mail server's answer. */
  testMailServer: (to?: string) => http.post('admin/settings/smtp/test', to ? { to } : {}),
}
