const FALLBACKS = ['.ts', '/index.ts'];

export async function resolve(specifier, context, next) {
  try {
    return await next(specifier, context);
  } catch (cause) {
    if (!specifier.startsWith('.')) {
      throw cause;
    }
    for (const suffix of FALLBACKS) {
      try {
        return await next(`${specifier}${suffix}`, context);
      } catch {
        continue;
      }
    }
    throw cause;
  }
}
