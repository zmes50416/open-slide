import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const desktopDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = path.resolve(desktopDir, '..', '..');
const shell = process.platform === 'win32';

const children = [
  spawn('pnpm', ['exec', 'turbo', 'run', 'dev', '--filter=demo'], {
    cwd: repoRoot,
    stdio: 'inherit',
    shell,
    env: { ...process.env, OPEN_SLIDE_SKIP_SKILLS_CHECK: '1' },
  }),
  spawn('pnpm', ['exec', 'vite'], {
    cwd: desktopDir,
    stdio: 'inherit',
    shell,
  }),
];

function shutdown(code) {
  for (const child of children) {
    if (child.exitCode === null) child.kill();
  }
  process.exit(code);
}

for (const child of children) {
  child.on('exit', (code) => shutdown(code ?? 0));
}
process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));
