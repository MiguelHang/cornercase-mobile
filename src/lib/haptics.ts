import type * as haptics from '@tauri-apps/plugin-haptics'

export type HapticsApi = Pick<typeof haptics, 'impactFeedback'>

export const tap = async (api: HapticsApi): Promise<void> => {
  try {
    await api.impactFeedback('light')
  } catch {
    return
  }
}
