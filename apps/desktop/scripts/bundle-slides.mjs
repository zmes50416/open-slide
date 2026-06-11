import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const desktopDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = path.resolve(desktopDir, '..', '..');
const slidesOut = path.join(desktopDir, 'dist', 'slides');

rmSync(slidesOut, { recursive: true, force: true });

const result = spawnSync('pnpm', ['exec', 'open-slide', 'build', '--out-dir', slidesOut], {
  cwd: path.join(repoRoot, 'apps', 'demo'),
  stdio: 'inherit',
  shell: process.platform === 'win32',
  env: {
    ...process.env,
    OPEN_SLIDE_BASE: '/slides/',
    OPEN_SLIDE_SKIP_SKILLS_CHECK: '1',
  },
});

process.exit(result.status ?? 1);
