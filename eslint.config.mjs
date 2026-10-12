import { createRequire } from 'node:module';

// FIXME: the root devDependencies still pin @typescript-eslint v3, which cannot
// load under ESLint 9 (it extends the removed CLIEngine). Until they are
// replaced by typescript-eslint v8, borrow the preview UI's copy; both trees are
// installed whenever the preview binary is built.
const requireFromPreviewUi = createRequire(new URL('./patto-preview-ui/package.json', import.meta.url));
const tseslint = requireFromPreviewUi('typescript-eslint');
const js = requireFromPreviewUi('@eslint/js');

export default tseslint.config(
	{ ignores: ['dist/**', 'node_modules/**', 'client/node_modules/**', 'patto-preview-ui/**', 'target/**'] },
	js.configs.recommended,
	...tseslint.configs.recommended,
	{ files: ['client/src/**/*.ts'] },
);
