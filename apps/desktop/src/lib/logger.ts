type LogLevel = 'error' | 'warn' | 'info' | 'debug';

const LEVEL_PRIORITY: Record<LogLevel, number> = {
  error: 0,
  warn: 1,
  info: 2,
  debug: 3,
};

let currentLevel: LogLevel = 'info';

function formatMessage(level: LogLevel, message: string, context?: Record<string, unknown>): string {
  const timestamp = new Date().toISOString();
  const ctx = context ? ` ${JSON.stringify(context)}` : '';
  return `[${timestamp}] [${level.toUpperCase()}] ${message}${ctx}`;
}

export const logger = {
  error: (message: string, context?: Record<string, unknown>) => {
    if (LEVEL_PRIORITY.error <= LEVEL_PRIORITY[currentLevel]) {
      console.error(formatMessage('error', message, context));
    }
  },
  warn: (message: string, context?: Record<string, unknown>) => {
    if (LEVEL_PRIORITY.warn <= LEVEL_PRIORITY[currentLevel]) {
      console.warn(formatMessage('warn', message, context));
    }
  },
  info: (message: string, context?: Record<string, unknown>) => {
    if (LEVEL_PRIORITY.info <= LEVEL_PRIORITY[currentLevel]) {
      console.info(formatMessage('info', message, context));
    }
  },
  debug: (message: string, context?: Record<string, unknown>) => {
    if (LEVEL_PRIORITY.debug <= LEVEL_PRIORITY[currentLevel]) {
      console.log(formatMessage('debug', message, context));
    }
  },
  setLevel: (level: LogLevel) => {
    currentLevel = level;
  },
};
