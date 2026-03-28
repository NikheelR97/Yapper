import tsParser from '@typescript-eslint/parser';
import globals from 'globals';

export default [
  {
    ignores: [
      '**/*.svelte',
      '.svelte-kit/**',
      'android/**',
      'build/**',
      'coverage/**',
      'dist/**',
      'node_modules/**',
      'playwright-report/**',
      'src-tauri/target/**',
      'test-results/**'
    ]
  },
  {
    files: ['**/*.{js,mjs,cjs,ts,mts,cts}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module'
      },
      globals: {
        ...globals.browser,
        ...globals.node
      }
    },
    rules: {}
  }
];
