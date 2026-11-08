import js from '@eslint/js'
import { defineConfig } from 'eslint/config'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import ts from 'typescript-eslint'

export default defineConfig(
  { ignores: ['dist/', 'src-tauri/', 'target/'] },
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
)
