/**
 * Unified logger abstraction layer.
 *
 * - Tauri desktop: routes logs through tauri-plugin-log (unified with Rust log pipeline)
 * - Web browser: falls back to console (extensible to remote logging services)
 *
 * 统一日志抽象层，为未来 Web 版本做好平台切换准备。
 * 业务代码只依赖 logger 接口，不直接调用 console.log。
 */

type LogFn = (...args: unknown[]) => void;

interface Logger {
  debug: LogFn;
  info: LogFn;
  warn: LogFn;
  error: LogFn;
}

/** Detect Tauri runtime by checking global flag injected by the Tauri WebView */
function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/** Stringify args into a single message string for Tauri plugin-log */
function formatArgs(args: unknown[]): string {
  return args
    .map((a) => {
      if (typeof a === 'string') return a;
      try {
        return JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(' ');
}

/**
 * Create a platform-adaptive logger.
 *
 * Tauri 环境：通过 tauri-plugin-log 输出（与 Rust log 统一管道，支持写入文件）
 * Web 环境：直接使用 console（后期可接入 Sentry / Datadog 等远程日志）
 */
function createLogger(): Logger {
  // Web / fallback implementation
  const consoleLogger: Logger = {
    debug: (...args) => console.debug('[DEBUG]', ...args),
    info: (...args) => console.info('[INFO]', ...args),
    warn: (...args) => console.warn('[WARN]', ...args),
    error: (...args) => console.error('[ERROR]', ...args),
  };

  if (!isTauri()) {
    return consoleLogger;
  }

  // Tauri: lazy-load plugin to avoid bundling Tauri deps in web builds
  let tauriLogModule: typeof import('@tauri-apps/plugin-log') | null = null;
  let loadPromise: Promise<typeof import('@tauri-apps/plugin-log') | null> | null = null;

  const loadTauriLog = (): Promise<typeof import('@tauri-apps/plugin-log') | null> => {
    if (tauriLogModule) return Promise.resolve(tauriLogModule);
    if (!loadPromise) {
      loadPromise = import('@tauri-apps/plugin-log')
        .then((mod) => {
          tauriLogModule = mod;
          return mod;
        })
        .catch(() => {
          // Plugin not available — fall back to console silently
          return null;
        });
    }
    return loadPromise;
  };

  const tauriLog = (
    level: 'debug' | 'info' | 'warn' | 'error',
    args: unknown[],
  ): void => {
    const msg = formatArgs(args);
    loadTauriLog().then((mod) => {
      if (mod) {
        mod[level](msg);
      } else {
        // Fallback when plugin failed to load
        consoleLogger[level](...args);
      }
    });
  };

  return {
    debug: (...args) => tauriLog('debug', args),
    info: (...args) => tauriLog('info', args),
    warn: (...args) => tauriLog('warn', args),
    error: (...args) => tauriLog('error', args),
  };
}

/** Singleton logger instance — import and use directly */
export const logger = createLogger();
