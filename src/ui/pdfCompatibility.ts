type PromiseConstructorWithTry = PromiseConstructor & {
  try?: <T>(callback: () => T | PromiseLike<T>) => Promise<T>;
};

type UrlConstructorWithParse = typeof URL & {
  parse?: (url: string, base?: string | URL) => URL | null;
};

export function ensurePdfWebViewCompatibility() {
  const compatiblePromise = Promise as PromiseConstructorWithTry;
  compatiblePromise.try ??= <T>(callback: () => T | PromiseLike<T>) =>
    Promise.resolve().then(callback);

  const compatibleUrl = URL as UrlConstructorWithParse;
  compatibleUrl.parse ??= (url: string, base?: string | URL) => {
    try {
      return new URL(url, base);
    } catch {
      return null;
    }
  };
}
