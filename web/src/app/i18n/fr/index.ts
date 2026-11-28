import account from './account.json' with { type: 'json' };
import accounts from './accounts.json' with { type: 'json' };
import common from './common.json' with { type: 'json' };
import files from './files.json' with { type: 'json' };
import history from './history.json' with { type: 'json' };
import publicLink from './public.json' with { type: 'json' };
import search from './search.json' with { type: 'json' };
import sharing from './sharing.json' with { type: 'json' };
import shell from './shell.json' with { type: 'json' };

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
};
