import axios from 'axios'
import { PublicConfigSchema } from '@/type/schemas'

export interface WriteBehindConfig {
  flushIntervalMs: number
  softLimitMiB: number
  hardLimitMiB: number
}

export interface AppConfig {
  // Rocket settings
  address: string
  port: number
  limits: Record<string, string>

  // App settings
  readOnlyMode: boolean
  disableImg: boolean
  writeBehind: WriteBehindConfig
  // password is handled separately now
  authKey: string | null
  hasAuthKey: boolean
  hasPassword: boolean
  hasDiscordHook: boolean
  discordHookUrl: string | null
  syncPaths: string[]
}

export const getConfig = async (): Promise<AppConfig> => {
  const response = await axios.get<AppConfig>('/get/config')
  const publicConfig = PublicConfigSchema.parse(response.data)
  return { ...response.data, ...publicConfig }
}

export const updateConfig = async (config: Partial<AppConfig>): Promise<void> => {
  await axios.put('/put/config', config)
}

export const updatePassword = async (oldPassword: string, newPassword?: string): Promise<void> => {
  await axios.put('/put/config/password', {
    oldPassword: oldPassword || null,
    password: newPassword
  })
}

export const exportConfig = async (): Promise<AppConfig> => {
  const response = await axios.get<AppConfig>('/get/config/export')
  return response.data
}

export const importConfig = async (config: AppConfig): Promise<void> => {
  // Refactor: path to /post/config/import
  await axios.post('/post/config/import', config)
}
