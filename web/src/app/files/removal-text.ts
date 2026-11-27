import { t } from '../i18n/language';
import type { NodeKind } from '../node';

export function removalBody(kind: NodeKind, ownFiles: boolean): string {
  if (kind === 'directory') {
    return ownFiles ? t('files.delete.folderOwn') : t('files.delete.folderShared');
  }
  return ownFiles ? t('files.delete.fileOwn') : t('files.delete.fileShared');
}
