import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { publishRelease } from '../scripts/publish-release.mjs';

const sha = 'a'.repeat(40);
function setup(t, { corrupt = false, moved = false, uploadFails = false } = {}) {
  const directory = mkdtempSync(join(tmpdir(), 'poster-maker-release-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  for (const name of ['Poster.Maker_0.3.2_aarch64.dmg', 'Poster.Maker_0.3.2_x64-setup.exe', 'SHA256SUMS.txt']) {
    writeFileSync(join(directory, name), `fixture: ${name}`);
  }
  const calls = [];
  const assets = new Map();
  let tagChecks = 0;
  const run = (command, args) => {
    calls.push({ command, args });
    if (command === 'git') {
      if (args[0] === 'fetch') return '';
      tagChecks++;
      return (moved && tagChecks === 2 ? 'b'.repeat(40) : sha) + '\n';
    }
    assert.equal(command, 'gh');
    if (args.includes('--paginate')) return '[[]]';
    if (args.includes('PATCH')) {
      assert.equal(args[1], 'repos/mics8128/poster-maker/releases/123');
      return JSON.stringify({ id: 123, tag_name: 'v0.3.2', draft: false, html_url: 'https://github.com/mics8128/poster-maker/releases/tag/v0.3.2' });
    }
    if (args[1].includes('uploads.github.com')) {
      if (uploadFails) throw new Error('upload interrupted');
      const url = new URL(args[1]);
      assert.equal(url.pathname, '/repos/mics8128/poster-maker/releases/123/assets');
      const bytes = readFileSync(args[args.indexOf('--input') + 1]);
      const id = assets.size + 1000;
      assets.set(id, bytes);
      return JSON.stringify({ id, name: url.searchParams.get('name'), size: bytes.length });
    }
    if (args[1].includes('/assets/')) {
      const id = Number(args[1].split('/').at(-1));
      return corrupt ? Buffer.from('wrong bytes') : assets.get(id);
    }
    assert.equal(args[1], 'repos/mics8128/poster-maker/releases');
    assert.ok(args.includes(`target_commitish=${sha}`));
    assert.ok(args.includes('draft=true'));
    return JSON.stringify({ id: 123, tag_name: 'v0.3.2', draft: true });
  };
  return { directory, run, calls };
}

test('uploads, downloads, and publishes only the exact new release and asset IDs', (t) => {
  const { directory, run, calls } = setup(t);
  assert.equal(publishRelease('mics8128/poster-maker', 'v0.3.2', sha, directory, run).id, 123);
  assert.equal(calls.filter(({ args }) => args.includes('--input')).length, 3);
  assert.equal(calls.filter(({ args }) => args[0] === 'rev-parse').length, 2);
  assert.equal(calls.at(-1).args[1], 'repos/mics8128/poster-maker/releases/123');
});

for (const [option, expected] of [['corrupt', /Checksum mismatch/], ['moved', /no longer points/], ['uploadFails', /upload interrupted/]]) {
  test(`${option} leaves the new draft unpublished and never deletes assets`, (t) => {
    const { directory, run, calls } = setup(t, { [option]: true });
    assert.throws(() => publishRelease('mics8128/poster-maker', 'v0.3.2', sha, directory, run), expected);
    assert.equal(calls.some(({ args }) => args.includes('PATCH') || args.includes('DELETE')), false);
  });
}

test('missing asset stops before creating a release', (t) => {
  const { directory, run, calls } = setup(t);
  rmSync(join(directory, 'Poster.Maker_0.3.2_aarch64.dmg'));
  assert.throws(() => publishRelease('mics8128/poster-maker', 'v0.3.2', sha, directory, run), /ENOENT/);
  assert.equal(calls.some(({ args }) => args.includes('POST')), false);
});
