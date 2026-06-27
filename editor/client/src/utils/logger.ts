export type LogLevel = "debug" | "info" | "warn" | "error";

interface LoggerOptions {
  prefix: string;
  minLevel?: LogLevel;
}

const LEVEL_RANK: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

function isValidLogLevel(value: unknown): value is LogLevel {
  return typeof value === "string" && value in LEVEL_RANK;
}

const envLevel = import.meta.env.VITE_LOG_LEVEL;
const GLOBAL_MIN_LEVEL: LogLevel = isValidLogLevel(envLevel) ? envLevel : "warn";

export class Logger {
  private prefix: string;
  private minLevel: number;

  constructor(opts: LoggerOptions) {
    this.prefix = opts.prefix;
    this.minLevel = LEVEL_RANK[opts.minLevel ?? GLOBAL_MIN_LEVEL];
  }

  private shouldLog(level: LogLevel): boolean {
    return LEVEL_RANK[level] >= this.minLevel;
  }

  debug(...args: unknown[]): void {
    if (this.shouldLog("debug")) console.debug(`[${this.prefix}]`, ...args);
  }

  info(...args: unknown[]): void {
    if (this.shouldLog("info")) console.info(`[${this.prefix}]`, ...args);
  }

  warn(...args: unknown[]): void {
    if (this.shouldLog("warn")) console.warn(`[${this.prefix}]`, ...args);
  }

  error(...args: unknown[]): void {
    if (this.shouldLog("error")) console.error(`[${this.prefix}]`, ...args);
  }
}
