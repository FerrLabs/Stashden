import { account } from './account';
import { accounts } from './accounts';
import { common } from './common';
import { files } from './files';
import { history } from './history';
import { publicLink } from './public';
import { search } from './search';
import { sharing } from './sharing';
import { shell } from './shell';

export const fr = {
  ...account,
  ...accounts,
  ...common,
  ...files,
  ...history,
  ...publicLink,
  ...search,
  ...sharing,
  ...shell,
} as const;
