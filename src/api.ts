import { Channel, invoke } from '@tauri-apps/api/core'

export type Harness = 'codex' | 'claude' | 'antigravity' | 'open_code'
export type Theme = 'dark' | 'light'

export interface Settings {
  source: string | null
  destinations: Partial<Record<Harness, string>>
  theme: Theme
}

export interface SettingsResponse {
  settings: Settings
  settingsFile: string
  error: string | null
}

export interface Difference {
  path: string
  kind: 'added' | 'removed' | 'changed' | 'type_changed'
}

export interface LinkWarning {
  path: string
  target: string
}

export interface SkillRow {
  folderName: string
  name: string | null
  description: string | null
  status: 'missing' | 'identical' | 'different' | 'installed_only' | 'invalid_source' | 'error' | 'ambiguous'
  sourcePath: string | null
  destinationPath: string | null
  warnings: string[]
  error: string | null
  differences: Difference[]
  linkWarnings: LinkWarning[]
}

export interface ScanResponse {
  revision: string
  harness: Harness
  sourcePath: string | null
  destinationPath: string
  skills: SkillRow[]
  warnings: string[]
}

export interface PreparedSkill {
  folderName: string
  linkWarnings: LinkWarning[]
  warnings: string[]
}

export interface PrepareResponse {
  token: string
  eligible: PreparedSkill[]
  skipped: string[]
  destinationPath: string
  warnings: string[]
}

export interface OperationEvent {
  kind: 'progress' | 'result' | 'finished'
  folderName: string | null
  completed: number
  total: number
  success: boolean | null
  message: string | null
}

export const loadSettings = () => invoke<SettingsResponse>('load_settings')
export const saveSettings = (settings: Settings) => invoke<void>('save_settings', { settings })
export const scanSkills = (harness: Harness) => invoke<ScanResponse>('scan_skills', { harness })
export const prepareInstall = (harness: Harness, revision: string, selected: string[]) =>
  invoke<PrepareResponse>('prepare_install', { harness, revision, selected })
export const executeInstall = (
  token: string,
  acknowledgeLinks: boolean,
  onEvent: (event: OperationEvent) => void,
) => {
  const channel = new Channel<OperationEvent>()
  channel.onmessage = onEvent
  return invoke<void>('execute_install', { token, acknowledgeLinks, onEvent: channel })
}
