import { mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const targetDir = process.env.CARGO_TARGET_DIR || join(tmpdir(), 'poster-maker-cargo-target');
mkdirSync(targetDir, { recursive: true });

const env = { ...process.env, CARGO_TARGET_DIR: targetDir };
const args = process.argv.slice(2);
const result = spawnSync('pnpm', ['exec', 'tauri', ...args], { stdio: 'inherit', env });

const isDev = args[0] === 'dev';
if (!isDev && process.env.POSTER_MAKER_KEEP_TARGET !== '1') {
  rmSync(targetDir, { recursive: true, force: true });
}

process.exit(result.status ?? 1);
