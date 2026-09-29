/// <reference types="vite-plus/test" />
import { defineConfig } from 'vite-plus'
import type { Plugin } from 'vite-plus'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'
import { createHash } from 'crypto'
import { visualizer } from 'rollup-plugin-visualizer'

const projectRoot = import.meta.dirname

const writeBuildId = (): Plugin => ({
  name: 'koel:build-id',
  apply: 'build',
  generateBundle(_options, bundle) {
    const buildId = createHash('sha256').update(Object.keys(bundle).sort().join('\n')).digest('hex').slice(0, 16)
    this.emitFile({ type: 'asset', fileName: 'build-id', source: buildId })
  },
})

export default defineConfig({
  staged: {
    'assets/**/*.{js,ts,css,pcss,vue}': ['vp check --fix'],
  },
  fmt: {
    semi: false,
    singleQuote: true,
    arrowParens: 'avoid',
    printWidth: 120,
    objectWrap: 'preserve',
    ignorePatterns: ['assets/css/vendor/**', 'assets/js/visualizers/**'],
  },
  lint: {
    plugins: ['typescript', 'vue', 'import'],
    categories: {
      correctness: 'error',
      suspicious: 'warn',
      pedantic: 'off',
      nursery: 'off',
      style: 'off',
      perf: 'warn',
      restriction: 'off',
    },
    rules: {
      'no-case-declarations': 'off',
      'no-new': 'off',
      'no-shadow': 'off',
      'no-unused-expressions': 'off',
      'no-unassigned-import': 'off',
      'no-await-in-loop': 'off',
      'typescript/ban-ts-comment': 'off',
      'no-unsafe-type-assertion': 'off',
      'no-unnecessary-type-assertion': 'off',
      'no-unnecessary-type-arguments': 'off',
      'no-unnecessary-type-parameters': 'off',
      'no-floating-promises': 'off',
      'unbound-method': 'off',
      'restrict-template-expressions': 'off',
      'no-redundant-type-constituents': 'off',
      'no-duplicate-type-constituents': 'off',
      'no-base-to-string': 'off',
      'await-thenable': 'off',
    },
    ignorePatterns: [
      'assets/tsconfig.json',
      'assets/css/vendor/**',
      'assets/js/visualizers/**',
    ],
    options: {
      typeAware: true,
      // TODO: enable typeCheck once tsgolint supports tsconfig paths resolution
      // typeCheck: true,
    },
  },
  plugins: [
    vue(),
    tailwindcss(),
    ...(process.env.VITEST
      ? []
      : [
          writeBuildId(),
          visualizer({
            filename: 'stats.html',
          }),
        ]),
  ],
  server: {
    // During development, píxiū runs on its own port; the player's API calls
    // and audio go there.
    proxy: {
      '/api': process.env.PIXIU_URL ?? 'http://127.0.0.1:4600',
      '/rest': process.env.PIXIU_URL ?? 'http://127.0.0.1:4600',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    cssMinify: 'esbuild',
    assetsInlineLimit: 0,
  },
  resolve: {
    alias: {
      '@': resolve(projectRoot, './assets/js'),
      '@modules': resolve(projectRoot, './node_modules'),
      lodash: 'lodash-es',
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: resolve(projectRoot, './assets/js/__tests__/setup.ts'),
    server: {
      deps: {
        cacheDir: resolve(projectRoot, 'node_modules/.vitest'),
      },
    },
  },
})
