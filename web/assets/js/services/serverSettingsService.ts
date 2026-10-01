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

export interface SingleSignOn {
  name: string
  issuer: string
  client_id: string
  /** The secret is never sent back; there always is one. */
  secret_set: boolean
  scopes: string[]
}

export interface SingleSignOnForm {
  name: string
  issuer: string
  client_id: string
  /** Left out keeps the stored one. */
  client_secret?: string
  scopes: string[]
}

export interface SingleSignOnReport {
  keys: number
  pkce_s256: boolean
}

export interface ServerSettings {
  public_url: string | null
  registration_open: boolean
  /** Whether email with links in it can go out: a mail server and the public address are set. */
  mail_ready: boolean
  smtp: MailServer | null
  oidc: SingleSignOn | null
  /** What to give the provider as the redirect URI; `null` without a public address. */
  redirect_uri: string | null
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
  setSingleSignOn: (form: SingleSignOnForm) => http.put<ServerSettings>('admin/settings/oidc', form),
  removeSingleSignOn: () => http.delete<ServerSettings>('admin/settings/oidc'),
  /** Checks a provider (saved or not); fails with what is wrong. */
  testSingleSignOn: (form: SingleSignOnForm) => http.post<SingleSignOnReport>('admin/settings/oidc/test', form),
  setRegistrationOpen: (open: boolean) => http.put<ServerSettings>('admin/settings/registration', { open }),
  /** Sends a test email now; fails with the mail server's answer. */
  testMailServer: (to?: string) => http.post('admin/settings/smtp/test', to ? { to } : {}),
}
