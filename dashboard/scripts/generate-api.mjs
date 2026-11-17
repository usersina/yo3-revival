import { spawnSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const dashboard = fileURLToPath(new URL('../', import.meta.url))
// Rust export is pure: no listener, configuration, game files or database is required.
const spec = spawnSync(
  'cargo',
  ['run', '--quiet', '--manifest-path', '../Cargo.toml', '-p', 'yo3-server', '--bin', 'openapi'],
  { cwd: dashboard, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 },
)
if (spec.status !== 0) {
  process.stderr.write(spec.stderr || String(spec.error))
  process.exit(spec.status ?? 1)
}
JSON.parse(spec.stdout)
writeFileSync(new URL('../api/openapi.json', import.meta.url), spec.stdout)
const generator = fileURLToPath(
  new URL('../node_modules/orval/dist/bin/orval.mjs', import.meta.url),
)
const result = spawnSync(
  process.execPath,
  [generator, '--config', 'orval.config.ts', '--fail-on-warnings'],
  {
    cwd: dashboard,
    stdio: 'inherit',
  },
)
process.exit(result.status ?? 1)
