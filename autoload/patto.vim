let s:root_dir = expand('<sfile>:h:h')
function! patto#init() abort
    if exists('g:lsp_loaded')
        " vim-lsp
        let l:script = s:root_dir . '/settings/vimlsp.vim'
        exe 'source ' l:script
        doautocmd <nomodeline> User lsp_setup
    endif
endfunction

