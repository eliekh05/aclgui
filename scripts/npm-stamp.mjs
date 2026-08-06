#!/usr/bin/env node
/**
 * Stamps per-platform npm packages from prebuilt binary artifacts.
 *
 * Usage:
 *   node scripts/npm-stamp.mjs --version=0.1.0 --binaries=dist/binaries [--out=dist/npm]
 *
 * Expects the following binary files in --binaries/:
 *   aclgui-darwin-arm64
 *   aclgui-darwin-x64
 *   aclgui-linux-arm64
 *   aclgui-linux-x64
 *   aclgui-win32-x64.exe
 *
 * Produces per-platform packages in --out/ ready to `npm publish`.
 * Platform packages are published first, then the metapackage.
 */

import { mkdirSync, copyFileSync, writeFileSync, chmodSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { execSync } from 'node:child_process';

const PLATFORMS = [
  { os: 'darwin', cpu: 'arm64', bin: 'aclgui-darwin-arm64',    ext: '' },
  { os: 'darwin', cpu: 'x64',   bin: 'aclgui-darwin-x64',      ext: '' },
  { os: 'linux',  cpu: 'arm64', bin: 'aclgui-linux-arm64',     ext: '' },
  { os: 'linux',  cpu: 'x64',   bin: 'aclgui-linux-x64',       ext: '' },
  { os: 'win32',  cpu: 'x64',   bin: 'aclgui-win32-x64.exe',   ext: '.exe' },
];

function parseArgs() {
  const args = Object.fromEntries(
    process.argv.slice(2)
      .filter(a => a.startsWith('--'))
      .map(a => { const [k, v] = a.slice(2).split('='); return [k, v]; })
  );
  if (!args.version) throw new Error('--version is required');
  return {
    version:  args.version,
    binaries: resolve(args.binaries ?? 'dist/binaries'),
    out:      resolve(args.out      ?? 'dist/npm'),
    publish:  'publish' in args,
    dryRun:   'dry-run' in args,
    tag:      args.tag ?? 'latest',
  };
}

function stampPlatform(p, { version, binaries, out }) {
  const src = join(binaries, p.bin);
  if (!existsSync(src)) {
    console.warn(`  skip @aclgui_eliekh05/${p.os}-${p.cpu}: binary not found (${src})`);
    return false;
  }

  const pkgName = `@aclgui_eliekh05/${p.os}-${p.cpu}`;
  const pkgDir  = join(out, `${p.os}-${p.cpu}`);
  mkdirSync(join(pkgDir, 'bin'), { recursive: true });

  copyFileSync(src, join(pkgDir, 'bin', `aclgui${p.ext}`));
  if (p.ext === '') chmodSync(join(pkgDir, 'bin', 'aclgui'), 0o755);

  writeFileSync(join(pkgDir, 'package.json'), JSON.stringify({
    name:        pkgName,
    version,
    description: `aclgui prebuilt binary for ${p.os}-${p.cpu}`,
    license:     'MIT',
    repository:  { type: 'git', url: 'https://github.com/eliekh05/aclgui.git' },
    os:          [p.os],
    cpu:         [p.cpu],
    files:       ['bin/'],
    bin:         { aclgui: `bin/aclgui${p.ext}` },
  }, null, 2));

  console.log(`  stamped ${pkgName}@${version}`);
  return true;
}

function stampMeta({ version, out }) {
  const metaSrc  = resolve('npm/aclgui');
  const metaOut  = join(out, 'aclgui');
  mkdirSync(join(metaOut, 'bin'), { recursive: true });

  // Copy bin shim
  copyFileSync(join(metaSrc, 'bin', 'aclgui.js'), join(metaOut, 'bin', 'aclgui.js'));
  chmodSync(join(metaOut, 'bin', 'aclgui.js'), 0o755);

  const optDeps = Object.fromEntries(
    PLATFORMS.map(p => [`@aclgui_eliekh05/${p.os}-${p.cpu}`, version])
  );

  writeFileSync(join(metaOut, 'package.json'), JSON.stringify({
    name:                 '@eliekh05/aclgui',
    version,
    description:          'Cross-platform ACL & permissions GUI — Windows, macOS, Linux',
    keywords:             ['acl', 'permissions', 'security', 'gui', 'chmod', 'icacls', 'setfacl'],
    license:              'MIT',
    repository:           { type: 'git', url: 'https://github.com/eliekh05/aclgui.git' },
    type:                 'module',
    bin:                  { aclgui: 'bin/aclgui.js' },
    files:                ['bin/'],
    engines:              { node: '>=20' },
    dependencies:         { 'bin-shim': '^0.1.0' },
    optionalDependencies: optDeps,
  }, null, 2));

  console.log(`  stamped aclgui@${version}`);
}

function npmPublish(pkgDir, { dryRun, tag }) {
  const args = ['npm', 'publish', pkgDir, '--access=public', `--tag=${tag}`];
  if (dryRun) args.push('--dry-run');
  console.log(`  $ ${args.join(' ')}`);
  execSync(args.join(' '), { stdio: 'inherit' });
}

const opts = parseArgs();
mkdirSync(opts.out, { recursive: true });

console.log(`Stamping platform packages → ${opts.out}`);
const stamped = PLATFORMS.map(p => ({ p, ok: stampPlatform(p, opts) }));

console.log(`Stamping metapackage`);
stampMeta(opts);

if (opts.publish) {
  console.log('\nPublishing platform packages first…');
  for (const { p, ok } of stamped) {
    if (!ok) continue;
    npmPublish(join(opts.out, `${p.os}-${p.cpu}`), opts);
  }
  console.log('\nPublishing metapackage…');
  npmPublish(join(opts.out, 'aclgui'), opts);
}

console.log('\nDone.');
