export type SyncCommand = 'sync-now' | 'pause' | 'resume' | 'stop';

export type SyncReport = {
  uploaded: number;
  downloaded: number;
  deleted_locally: number;
  deleted_remotely: number;
  conflicts: string[];
  blocked: string[];
  held: string[];
  unsupported: { path: string; reason: string }[];
  skipped: string[];
  failures: { path: string; reason: string }[];
};

export type SyncStatus =
  | { state: 'idle' }
  | { state: 'syncing' }
  | ({ state: 'synced' } & SyncReport)
  | { state: 'failed'; reason: string }
  | { state: 'paused' }
  | { state: 'stopped' };

export type Syncing = {
  folder: string;
  status: SyncStatus;
  last: SyncReport | null;
};

export function summarise(report: SyncReport): string {
  return (
    `${report.uploaded} up, ${report.downloaded} down, ` +
    `${report.deleted_locally} deleted here, ${report.deleted_remotely} deleted on the server`
  );
}

export function describe(status: SyncStatus): string {
  switch (status.state) {
    case 'idle':
      return 'Watching for changes';
    case 'syncing':
      return 'Syncing';
    case 'synced':
      return 'In step with the server';
    case 'failed':
      return `The last pass failed: ${status.reason}`;
    case 'paused':
      return 'Paused, changes wait until you resume';
    case 'stopped':
      return 'Stopped';
  }
}

export function needsAttention(report: SyncReport): boolean {
  return (
    report.conflicts.length > 0 ||
    report.blocked.length > 0 ||
    report.held.length > 0 ||
    report.unsupported.length > 0 ||
    report.skipped.length > 0 ||
    report.failures.length > 0
  );
}
