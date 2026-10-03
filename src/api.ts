import { Channel, invoke } from '@tauri-apps/api/core'

export type Harness = string
export type Theme = 'dark' | 'light'
export type OperationAction = 'install' | 'update' | 'uninstall'

export interface HarnessDescriptor {
  id: Harness
  label: string
  builtIn: boolean
  visible: boolean
  destinationAvailable: boolean
}

export interface Settings {
  source: string | null
  destinations: Record<Harness, string>
  custom_harnesses: Record<Harness, string>
  theme: Theme
}

export interface SettingsResponse {
  settings: Settings
  settingsFile: string
  error: string | null
  harnesses: HarnessDescriptor[]
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
  eligibleActions: OperationAction[]
}

export interface ScanProgress {
  stage: 'discover_source' | 'source' | 'discover_destination' | 'destination' | 'compare'
  completed: number
  total: number | null
  reusedSource: boolean
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
  action: OperationAction
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
export const openHarnessFolder = (harness: Harness) => invoke<void>('open_harness_folder', { harness })
export const scanSkills = (harness: Harness, reuseSource = false, onProgress?: (event: ScanProgress) => void) => {
  const channel = onProgress ? new Channel<ScanProgress>() : undefined
  if (channel && onProgress) channel.onmessage = onProgress
  return invoke<ScanResponse>('scan_skills', { harness, reuseSource, onProgress: channel ?? null })
}
export const prepareOperation = (action: OperationAction, harness: Harness, revision: string, selected: string[]) =>
  invoke<PrepareResponse>('prepare_operation', { action, harness, revision, selected })
export const executeOperation = (
  token: string,
  acknowledgeLinks: boolean,
  onEvent: (event: OperationEvent) => void,
) => {
  const channel = new Channel<OperationEvent>()
  channel.onmessage = onEvent
  return invoke<void>('execute_operation', { token, acknowledgeLinks, onEvent: channel })
}
