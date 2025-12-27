import axios from 'axios'

// Refactor: Renamed AppSettings to AppConfig to match backend
export interface AppConfig {
  readOnlyMode: boolean
  disableImg: boolean
  password: string
  authKey?: string | null
  discordHookUrl?: string | null
  syncPaths: string[]
  uploadLimitMb: number
}

export const getConfig = async (): Promise<AppConfig> => {
  // Refactor: path to /get/config
  const response = await axios.get<AppConfig>('/get/config')
  return response.data
}

export const updateConfig = async (config: AppConfig): Promise<void> => {
  // Refactor: path to /put/config
  await axios.put('/put/config', config)
}

export const exportConfig = async (): Promise<AppConfig> => {
  // Refactor: path to /get/config/export
  const response = await axios.get<AppConfig>('/get/config/export')
  return response.data
}

export const importConfig = async (config: AppConfig): Promise<void> => {
  // Refactor: path to /post/config/import
  await axios.post('/post/config/import', config)
}
