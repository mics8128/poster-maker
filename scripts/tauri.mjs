import { spawnSync } from 'node:child_process';
import { homedir } from 'node:os';
import { join } from 'node:path';

const defaultTargetDir = join(homedir(), 'Library', 'Caches', 'poster-maker', 'cargo-target');
const env = {
  ...process.env,
  CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || defaultTargetDir,
};

const result = spawnSync('pnpm', ['exec', 'tauri', ...process.argv.slice(2)], {
  stdio: 'inherit',
  env,
});

process.exit(result.status ?? 1);
