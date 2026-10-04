import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = fileURLToPath(new URL('..', import.meta.url));
const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*)?$/;

export function validateVersion(version) {
  if (typeof version !== 'string' || !versionPattern.test(version)) {
    throw new Error(`Invalid release version: ${JSON.stringify(version)}`);
  }
  return version;
}

function replaceOne(text, pattern, replacement, label) {
  const matches = [...text.matchAll(new RegExp(pattern.source, 'gm'))];
  if (matches.length !== 1) throw new Error(`Expected one ${label}, found ${matches.length}`);
  return text.replace(pattern, replacement);
}

export function syncVersion(root = projectRoot, { check = false, tag } = {}) {
  const read = (path) => readFileSync(resolve(root, path), 'utf8');
  const version = validateVersion(JSON.parse(read('package.json')).version);
  if (tag !== undefined && tag !== `v${version}`) {
    throw new Error(`Release tag ${JSON.stringify(tag)} does not match package version v${version}`);
  }

  // Only update this package, never dependency versions or lockfile resolution.
  const cargo = read('src-tauri/Cargo.toml');
  const lock = read('src-tauri/Cargo.lock');
  const tauriText = read('src-tauri/tauri.conf.json');
  const tauri = JSON.parse(tauriText);
  const updates = new Map([
    ['src-tauri/Cargo.toml', replaceOne(cargo, /(^\[package\][\s\S]*?\nversion = ")[^"]+("\r?$)/m, `$1${version}$2`, 'Cargo package version')],
    ['src-tauri/Cargo.lock', replaceOne(lock, /(^name = "poster-maker"\r?\nversion = ")[^"]+("\r?$)/m, `$1${version}$2`, 'Cargo lock package version')],
  ]);
  tauri.version = version;
  for (const windowConfig of tauri.app?.windows ?? []) {
    if (typeof windowConfig.title === 'string') {
      windowConfig.title = windowConfig.title.replace(/ v\d+\.\d+\.\d+(?:-[\w.-]+)?$/, ` v${version}`);
    }
  }
  const tauriJson = `${JSON.stringify(tauri, null, 2)}\n`;
  updates.set('src-tauri/tauri.conf.json', tauriText.includes('\r\n') ? tauriJson.replace(/\n/g, '\r\n') : tauriJson);

  const changed = [...updates].filter(([path, content]) => read(path) !== content);
  if (check && changed.length) {
    throw new Error(`Version files are out of sync: ${changed.map(([path]) => path).join(', ')}. Run pnpm sync-version.`);
  }
  // Validate every file before writing any of them.
  if (!check) for (const [path, content] of changed) writeFileSync(resolve(root, path), content);
  return version;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    if (args.some((arg) => arg !== '--check' && !arg.startsWith('--tag='))) {
      throw new Error('Usage: node scripts/sync-version.mjs [--check] [--tag=vX.Y.Z]');
    }
    const version = syncVersion(projectRoot, {
      check: args.includes('--check'),
      tag: args.find((arg) => arg.startsWith('--tag='))?.slice(6),
    });
    console.log(`${args.includes('--check') ? 'Checked' : 'Synced'} version ${version}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
