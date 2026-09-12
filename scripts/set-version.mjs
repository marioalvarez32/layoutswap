// Writes one version into the two files pnpm does not own: the Tauri config, which
// names the installers, and the Rust crate. semantic-release runs this before the
// build so the installers and the About box carry the version it just decided;
// package.json is bumped by its npm plugin. Usage: node scripts/set-version.mjs 0.2.0
import { readFileSync, writeFileSync } from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
  console.error(`set-version: expected a semantic version, got ${JSON.stringify(version)}`);
  process.exit(2);
}

const tauriConf = 'src-tauri/tauri.conf.json';
const conf = JSON.parse(readFileSync(tauriConf, 'utf8'));
conf.version = version;
writeFileSync(tauriConf, `${JSON.stringify(conf, null, 2)}\n`);

const cargoToml = 'src-tauri/Cargo.toml';
const toml = readFileSync(cargoToml, 'utf8');
const bumped = toml.replace(/^version = "[^"]*"/m, `version = "${version}"`);
if (bumped === toml) {
  console.error(`set-version: no version line found in ${cargoToml}`);
  process.exit(2);
}
writeFileSync(cargoToml, bumped);

console.log(`set-version: ${tauriConf} and ${cargoToml} now say ${version}`);
