import { defineConfig } from '@playwright/test';
import { existsSync } from 'node:fs';

// The dev container ships a Chromium build; use it directly if present.
const exe = [process.env.CHROMIUM_PATH, '/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell'].find((p) => p && existsSync(p));

export default defineConfig({
  testDir: './tests',
  use: {
    baseURL: 'http://localhost:4173',
    launchOptions: exe ? { executablePath: exe } : {},
  },
  webServer: {
    command: 'npx vite preview --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: false,
  },
});
