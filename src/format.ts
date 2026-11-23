export const differenceLabels = { added: 'Will be added to destination', removed: 'Will be removed from destination', changed: 'Content or permissions changed', type_changed: 'Entry type changed' }

export function statusLabel(status: string) {
  return ({
    missing: 'Missing',
    identical: 'Identical',
    different: 'Different',
    installed_only: 'Installed only',
    invalid_source: 'Invalid source',
    ambiguous: 'Ambiguous',
    error: 'Scan error',
  } as Record<string, string>)[status] ?? status
}

export function formatPath(path: string | null | undefined) {
  return path || 'Not configured'
}
