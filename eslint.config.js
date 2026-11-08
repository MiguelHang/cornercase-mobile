import js from '@eslint/js'
import { defineConfig } from 'eslint/config'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import ts from 'typescript-eslint'

export default defineConfig(
  { ignores: ['dist/', 'src-tauri/', 'target/', 'src/lib/bindings.ts'] },
  js.configs.recommended,
  ts.configs.recommended,
  svelte.configs['flat/recommended'],
  {
    languageOptions: { globals: globals.browser },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
  {
    files: ['**/*.{ts,js,mjs,svelte}'],
    rules: {
      'max-lines-per-function': ['error', { max: 40, skipBlankLines: true, skipComments: true }],
      complexity: ['error', 10],
      'max-depth': ['error', 3],
    },
  },
  {
    files: ['scripts/**'],
    languageOptions: { globals: globals.node },
  },
  {
    files: ['**/*.test.ts'],
    rules: { 'max-lines-per-function': 'off' },
  },
)
