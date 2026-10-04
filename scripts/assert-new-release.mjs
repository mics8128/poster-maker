import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { validateVersion } from './sync-version.mjs';

export function assertNewRelease(repository, tag, run = execFileSync) {
  if (!/^[A-Za-z0-9][A-Za-z0-9-]*\/[A-Za-z0-9_.-]+$/.test(repository ?? '')) throw new Error('Invalid repository');
  if (!tag?.startsWith('v')) throw new Error('Release tags must start with v');
  validateVersion(tag.slice(1));
  let pages;
  try {
    pages = JSON.parse(run('gh', ['api', '--paginate', '--slurp', `repos/${repository}/releases?per_page=100`], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }));
  } catch (error) {
    throw new Error(`Cannot verify whether ${tag} already exists: ${String(error.stderr || error.message).trim()}`);
  }
  if (!Array.isArray(pages) || pages.some((page) => !Array.isArray(page))) {
    throw new Error('Cannot verify existing releases: unexpected GitHub response');
  }
  // The tag endpoint omits drafts. List every page so old and draft releases are protected too.
  if (pages.flat().some((release) => release.tag_name === tag)) {
    throw new Error(`Release ${tag} already exists. Existing releases and assets will not be changed; use a new patch version.`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assertNewRelease(process.env.GITHUB_REPOSITORY, process.env.RELEASE_TAG);
    console.log(`Release ${process.env.RELEASE_TAG} is new`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
