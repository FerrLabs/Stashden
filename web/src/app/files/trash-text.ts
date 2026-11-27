import { formatMoment, formatSize, joinDetails, t } from '../i18n/language';
import type { Trashed } from '../node';

export function describeTrashed(node: Trashed): string {
  const parts = [node.kind === 'directory' ? t('common.folder') : formatSize(node.size)];
  if (node.deleted_at !== undefined) {
    parts.push(t('trash.deleted', { moment: formatMoment(node.deleted_at) }));
  }
  if (node.expires_at !== undefined) {
    parts.push(t('trash.expires', { moment: formatMoment(node.expires_at) }));
  }
  return joinDetails(parts);
}
