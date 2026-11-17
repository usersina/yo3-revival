import { defineConfig } from 'orval'

const header = () => [
  'Generated from Rust endpoints by Orval. DO NOT EDIT.',
  'Regenerate: npm run api:generate',
]

export default defineConfig({
  dashboard: {
    input: './api/openapi.json',
    output: {
      target: './src/api/generated/http/client.ts',
      clean: true,
      schemas: './src/api/generated/http/models',
      client: 'react-query',
      httpClient: 'fetch',
      headers: true,
      override: {
        header,
        mutator: { path: './src/api/transport.ts', name: 'apiFetch' },
        fetch: { includeHttpResponseReturnType: false },
        query: { signal: true },
      },
    },
  },
  validators: {
    input: './api/openapi.json',
    output: {
      target: './src/api/generated/validation/schemas.ts',
      clean: true,
      client: 'zod',
      override: {
        header,
        zod: { version: 4, generateEachHttpStatus: true, generateReusableSchemas: true },
      },
    },
  },
})
