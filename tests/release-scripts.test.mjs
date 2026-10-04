import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { syncVersion, validateVersion } from '../scripts/sync-version.mjs';
import { assertNewRelease } from '../scripts/assert-new-release.mjs';

function fixture(t, version = '0.3.2') {
  const root = mkdtempSync(join(tmpdir(), 'poster-maker-version-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'src-tauri'));
  const write = (path, value) => writeFileSync(join(root, path), value);
  const read = (path) => readFileSync(join(root, path), 'utf8');
  write('package.json', JSON.stringify({ version }));
  write('src-tauri/Cargo.toml', '[package]\nname = "poster-maker"\nversion = "0.3.1"\n\n[dependencies]\nother = "1.2.3"\n');
  write('src-tauri/Cargo.lock', 'version = 4\n\n[[package]]\nname = "other"\nversion = "1.2.3"\n\n[[package]]\nname = "poster-maker"\nversion = "0.3.1"\ndependencies = ["other"]\n');
  write('src-tauri/tauri.conf.json', JSON.stringify({ version: '0.3.1', app: { windows: [{ title: 'Poster Maker v0.3.1' }, { title: 'Other window' }] } }));
  return { root, read, write };
}

test('syncs Cargo, lockfile, Tauri version and window title without changing dependencies', (t) => {
  const { root, read } = fixture(t);
  assert.equal(syncVersion(root), '0.3.2');
  assert.match(read('src-tauri/Cargo.toml'), /version = "0.3.2"/);
  assert.match(read('src-tauri/Cargo.toml'), /other = "1.2.3"/);
  assert.match(read('src-tauri/Cargo.lock'), /name = "poster-maker"\nversion = "0.3.2"/);
  assert.match(read('src-tauri/Cargo.lock'), /name = "other"\nversion = "1.2.3"/);
  const tauri = JSON.parse(read('src-tauri/tauri.conf.json'));
  assert.equal(tauri.version, '0.3.2');
  assert.equal(tauri.app.windows[0].title, 'Poster Maker v0.3.2');
  assert.equal(tauri.app.windows[1].title, 'Other window');
  assert.equal(syncVersion(root, { check: true, tag: 'v0.3.2' }), '0.3.2');
});

test('check mode rejects drift without rewriting files', (t) => {
  const { root, read } = fixture(t);
  const before = read('src-tauri/Cargo.lock');
  assert.throws(() => syncVersion(root, { check: true }), /out of sync/);
  assert.equal(read('src-tauri/Cargo.lock'), before);
});

test('rejects a release tag that differs from the committed version', (t) => {
  const { root } = fixture(t);
  assert.throws(() => syncVersion(root, { tag: 'v9.9.9' }), /does not match/);
});

test('missing lockfile package fails before any write', (t) => {
  const { root, read, write } = fixture(t);
  const before = read('src-tauri/Cargo.toml');
  write('src-tauri/Cargo.lock', 'version = 4\n');
  assert.throws(() => syncVersion(root), /Expected one Cargo lock package version/);
  assert.equal(read('src-tauri/Cargo.toml'), before);
});

test('duplicate lockfile package fails before any write', (t) => {
  const { root, read, write } = fixture(t);
  write('src-tauri/Cargo.lock', read('src-tauri/Cargo.lock') + '\n[[package]]\nname = "poster-maker"\nversion = "0.1.0"\n');
  assert.throws(() => syncVersion(root), /found 2/);
});

test('syncs valid prerelease versions', (t) => {
  const { root, read } = fixture(t, '0.3.2-rc.1');
  syncVersion(root);
  assert.equal(JSON.parse(read('src-tauri/tauri.conf.json')).version, '0.3.2-rc.1');
  assert.equal(syncVersion(root, { check: true, tag: 'v0.3.2-rc.1' }), '0.3.2-rc.1');
});

for (const version of ['', 'v0.3.2', '0.3', '00.3.2', '0.3.2-01', '0.3.2\nmalicious', null]) {
  test(`rejects invalid version ${JSON.stringify(version)}`, () => assert.throws(() => validateVersion(version), /Invalid/));
}

for (const draft of [false, true]) {
  test(`refuses an existing ${draft ? 'draft' : 'published'} release on any page`, () => {
    assert.throws(() => assertNewRelease('mics8128/poster-maker', 'v0.3.2', () => JSON.stringify([[{ tag_name: 'v0.3.1' }], [{ tag_name: 'v0.3.2', draft }]])), /already exists/);
  });
}

test('only a successful complete listing permits a new release', () => {
  let called = false;
  assertNewRelease('mics8128/poster-maker', 'v0.3.2', (command, args) => {
    called = true;
    assert.equal(command, 'gh');
    assert.deepEqual(args, ['api', '--paginate', '--slurp', 'repos/mics8128/poster-maker/releases?per_page=100']);
    return JSON.stringify([[{ tag_name: 'v0.3.1' }], []]);
  });
  assert.equal(called, true);
});

test('rejects malformed API responses', () => {
  for (const response of ['invalid JSON', '{}', '[{}]']) {
    assert.throws(() => assertNewRelease('mics8128/poster-maker', 'v0.3.2', () => response), /Cannot verify/);
  }
});

for (const stderr of ['gh: Not Found (HTTP 404)', 'gh: Bad credentials (HTTP 401)', 'gh: Forbidden (HTTP 403)', 'connection reset', 'gh: Server error (HTTP 500)']) {
  test(`fails closed on ${stderr}`, () => {
    assert.throws(() => assertNewRelease('mics8128/poster-maker', 'v0.3.2', () => {
      throw Object.assign(new Error('request failed'), { status: 1, stderr });
    }), /Cannot verify/);
  });
}

test('invalid tag or repository never calls GitHub', () => {
  const never = () => assert.fail('GitHub should not be called');
  assert.throws(() => assertNewRelease('mics8128/poster-maker', 'master', never), /start with v/);
  assert.throws(() => assertNewRelease('mics8128/poster-maker', 'v0.3.2/other', never), /Invalid/);
  assert.throws(() => assertNewRelease('../bad', 'v0.3.2', never), /Invalid repository/);
});

test('correct CRLF checkout passes --check on Windows without rewriting', (t) => {
  const { root, read, write } = fixture(t);
  syncVersion(root);
  for (const path of ['src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json']) {
    write(path, read(path).replace(/\n/g, '\r\n'));
  }
  const before = read('src-tauri/tauri.conf.json');
  assert.equal(syncVersion(root, { check: true }), '0.3.2');
  assert.equal(read('src-tauri/tauri.conf.json'), before);
});
