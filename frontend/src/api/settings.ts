import axios from 'axios'

export interface AppSettings {
  readOnlyMode: boolean
  disableImg: boolean
  password: string
  authKey?: string | null
  discordHookUrl?: string | null
  syncPaths: string[]
  uploadLimitMb: number
}

export const getSettings = async (): Promise<AppSettings> => {
  // 修改為 /get/settings
  const response = await axios.get<AppSettings>('/get/settings')
  return response.data
}

export const updateSettings = async (settings: AppSettings): Promise<void> => {
  // 修改為 /put/settings
  await axios.put('/put/settings', settings)
}

export const exportSettings = async (): Promise<AppSettings> => {
  // 修改為 /get/settings/export
  const response = await axios.get<AppSettings>('/get/settings/export')
  return response.data
}

export const importSettings = async (settings: AppSettings): Promise<void> => {
  // 修改為 /post/settings/import
  await axios.post('/post/settings/import', settings)
}
