import { readFileSync, writeFileSync } from 'node:fs';

const version = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version;

const cargoPath = new URL('../src-tauri/Cargo.toml', import.meta.url);
let cargo = readFileSync(cargoPath, 'utf8');
cargo = cargo.replace(/^version = "[^"]+"/m, `version = "${version}"`);
writeFileSync(cargoPath, cargo);

const tauriPath = new URL('../src-tauri/tauri.conf.json', import.meta.url);
const tauri = JSON.parse(readFileSync(tauriPath, 'utf8'));
tauri.version = version;
for (const windowConfig of tauri.app?.windows ?? []) {
  if (typeof windowConfig.title === 'string') {
    windowConfig.title = windowConfig.title.replace(/ v\d+\.\d+\.\d+(?:-[\w.]+)?$/, ` v${version}`);
  }
}
writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

console.log(`Synced version ${version}`);
