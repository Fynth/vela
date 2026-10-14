import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  timeout: 60000,
  expect: { timeout: 10000 },
  workers: 1,
  fullyParallel: false,
  reporter: 'list',
  use: { baseURL: 'http://127.0.0.1:3217', headless: true, trace: 'retain-on-failure', screenshot: 'only-on-failure' },
  webServer: { command: 'bash server.sh', url: 'http://127.0.0.1:3217/health', reuseExistingServer: false, timeout: 30000, gracefulShutdown: { signal: 'SIGTERM', timeout: 20000 } },
});
