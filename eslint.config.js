const js = require('@eslint/js');
const prettier = require('eslint-config-prettier');

module.exports = [
  js.configs.recommended,
  prettier,
  {
    files: ['pages/**/*.js'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        document: 'readonly',
        localStorage: 'readonly',
        console: 'readonly',
        WebAssembly: 'readonly',
        Uint8Array: 'readonly',
        FileReader: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        Event: 'readonly',
        Promise: 'readonly',
      },
    },
  },
  {
    files: ['*.config.js'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'commonjs',
      globals: {
        module: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        process: 'readonly',
      },
    },
  },
  {
    ignores: ['_site/', 'pkg/', 'target/'],
  },
];
