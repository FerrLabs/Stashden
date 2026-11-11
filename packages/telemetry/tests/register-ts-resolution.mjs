import { registerHooks } from 'node:module';

registerHooks({
  resolve(specifier, context, nextResolve) {
    const relative = specifier.startsWith('./') || specifier.startsWith('../');
    if (context.parentURL?.includes('/node_modules/') || !relative || /\.[cm]?[jt]s$/.test(specifier)) {
      return nextResolve(specifier, context);
    }
    return nextResolve(`${specifier}.ts`, context);
  },
});
