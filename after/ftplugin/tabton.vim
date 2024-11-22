setlocal suffixesadd=.tb

" hard tab is important in tabton
setlocal noexpandtab

" conceal+wrap in vim/neovim is still weird
" see https://github.com/vim/vim/pull/10442
setlocal nowrap
setlocal commentstring=[-\ %s]
