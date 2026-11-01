/** Strict typed linting for handwritten frontend code; generated declarations are checked by tsc. */
import js from '@eslint/js';
import { defineConfig } from 'eslint/config';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import jsdoc from 'eslint-plugin-jsdoc';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

export default defineConfig(
  {
    ignores: [
      'node_modules/**',
      '.svelte-kit/**',
      'build/**',
      'src/lib/api/generated/**',
    ],
  },
  {
    files: [
      'src/**/*.ts',
      'src/**/*.svelte',
      'test/**/*.ts',
      'scripts/**/*.ts',
      'vite.config.ts',
    ],
    extends: [
      js.configs.recommended,
      ...ts.configs.strictTypeChecked,
      ...ts.configs.stylisticTypeChecked,
    ],
    languageOptions: {
      parser: ts.parser,
      parserOptions: { projectService: true, extraFileExtensions: ['.svelte'] },
      globals: { ...globals.browser, ...globals.node },
    },
    plugins: { jsdoc },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/strict-boolean-expressions': 'error',
      '@typescript-eslint/explicit-module-boundary-types': 'error',
      '@typescript-eslint/switch-exhaustiveness-check': 'error',
      'jsdoc/require-jsdoc': [
        'error',
        {
          contexts: [
            'FunctionDeclaration',
            'TSInterfaceDeclaration',
            'TSTypeAliasDeclaration',
          ],
          require: {
            FunctionDeclaration: true,
            ClassDeclaration: true,
            MethodDefinition: true,
          },
        },
      ],
      'no-warning-comments': [
        'error',
        { terms: ['todo', 'fixme'], location: 'start' },
      ],
    },
  },
  ...svelte.configs.recommended,
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        projectService: true,
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  prettier,
);
