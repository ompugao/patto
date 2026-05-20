setlocal suffixesadd=.pn

" hard tab is important in patto
setlocal noexpandtab

" conceal+wrap in vim/neovim is still weird
" see https://github.com/vim/vim/pull/10442
setlocal nowrap
setlocal commentstring=[-\ %s]

" LSP-based folding (requires Neovim 0.10+ with patto-lsp running)
if has('nvim-0.10')
  setlocal foldmethod=expr
  setlocal foldexpr=v:lua.vim.lsp.foldexpr()
  setlocal foldtext=v:lua.require('patto.foldtext').foldtext()
  setlocal foldlevel=99
endif
