import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { assertNewRelease } from './assert-new-release.mjs';

export function publishRelease(repository, tag, sha, directory, run = execFileSync) {
  if (!/^[a-f0-9]{40}$/.test(sha ?? '')) throw new Error('Expected an exact commit SHA');
  assertNewRelease(repository, tag, run);
  const version = tag.slice(1);
  const filenames = [`Poster.Maker_${version}_aarch64.dmg`, `Poster.Maker_${version}_x64-setup.exe`, 'SHA256SUMS.txt'];
  const files = filenames.map((name) => ({ name, path: resolve(directory, name), bytes: readFileSync(resolve(directory, name)) }));
  if (files.some(({ bytes }) => bytes.length === 0)) throw new Error('Release assets must not be empty');
  const api = (args) => JSON.parse(run('gh', ['api', ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }));
  const endpoint = `repos/${repository}/releases`;
  const assertTagCommit = () => {
    run('git', ['fetch', '--no-tags', 'origin', `refs/tags/${tag}`], { stdio: ['ignore', 'pipe', 'pipe'] });
    const actual = run('git', ['rev-parse', 'FETCH_HEAD^{commit}'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
    if (actual !== sha) throw new Error(`Tag ${tag} no longer points to ${sha}; publication stopped`);
  };
  assertTagCommit();
  const release = api([endpoint, '--method', 'POST', '-f', `tag_name=${tag}`, '-f', `target_commitish=${sha}`,
    '-f', `name=Poster Maker ${tag}`, '-F', 'draft=true', '-F', `prerelease=${tag.includes('-')}`, '-F', 'generate_release_notes=true']);
  if (!Number.isSafeInteger(release.id) || release.tag_name !== tag || !release.draft) {
    throw new Error('Unexpected draft response; inspect the release before retrying');
  }
  console.log(`Created new draft release ${release.id}`);
  // Use the returned release/asset IDs throughout. Never resolve a draft by tag again:
  // another maintainer creating a same-tag draft cannot make us edit their release.
  for (const file of files) {
    const asset = api([`https://uploads.github.com/repos/${repository}/releases/${release.id}/assets?name=${encodeURIComponent(file.name)}`,
      '--method', 'POST', '--header', 'Content-Type: application/octet-stream', '--input', file.path]);
    if (!Number.isSafeInteger(asset.id) || asset.name !== file.name || asset.size !== file.bytes.length) {
      throw new Error(`Unexpected upload response for ${file.name}; draft ${release.id} remains unpublished`);
    }
    const downloaded = run('gh', ['api', `repos/${repository}/releases/assets/${asset.id}`, '--header', 'Accept: application/octet-stream'],
      { stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: Math.max(file.bytes.length * 2, 1024 * 1024) });
    const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
    if (digest(downloaded) !== digest(file.bytes)) throw new Error(`Checksum mismatch for ${file.name}; draft ${release.id} remains unpublished`);
    console.log(`Verified ${file.name}: ${digest(downloaded)}`);
  }
  assertTagCommit();
  const published = api([`${endpoint}/${release.id}`, '--method', 'PATCH', '-F', 'draft=false']);
  if (published.id !== release.id || published.tag_name !== tag || published.draft) throw new Error(`Verify publication state of release ${release.id}`);
  return published;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const release = publishRelease(process.env.GITHUB_REPOSITORY, process.env.RELEASE_TAG, process.env.RELEASE_SHA, process.argv[2] ?? 'release-files');
    console.log(`Published ${release.html_url}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
