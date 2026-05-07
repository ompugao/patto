" Port of lua/patto_preview_toggle.lua
"
" Toggle patto-preview-tui in a zoomed tmux pane with viewport sync.
"
" Usage (add to your vimrc):
"   nnoremap <leader>p :call patto_preview_toggle#toggle()<CR>
"
" For viewport sync, configure patto-preview-tui.toml like this:
"   [editor]
"   cmd = '''vim --servername "$VIM_SERVERNAME" --remote "{file}" && vim --servername "$VIM_SERVERNAME" --remote-expr "patto_preview_toggle#schedule_restore({top_line}, {line})"'''
"   action = "quit"

" ---------------------------------------------------------------------------
" patto_preview_toggle#toggle()
" Open patto-preview-tui for the current buffer in a zoomed tmux pane.
" ---------------------------------------------------------------------------
function! patto_preview_toggle#toggle() abort
    if empty($TMUX)
        echohl WarningMsg
        echomsg 'patto_preview_toggle: not inside tmux'
        echohl None
        return
    endif

    let l:file = expand('%:p')
    if l:file ==# ''
        echohl WarningMsg
        echomsg 'patto_preview_toggle: no file in current buffer'
        echohl None
        return
    endif

    let l:topline = line('w0')
    let l:binary  = get(g:, 'patto_preview_tui_binary', 'patto-preview-tui')
    let l:extra   = get(g:, 'patto_preview_tui_extra_args', [])

    let l:cmd_parts = [shellescape(l:binary), shellescape(l:file),
                \      '--goto-line', l:topline]
    for l:arg in l:extra
        call add(l:cmd_parts, shellescape(l:arg))
    endfor
    let l:tui_cmd = join(l:cmd_parts, ' ')

    " Pass $VIM_SERVERNAME so the TUI's editor command can reach this Vim
    " instance via --remote / --remote-expr.
    call system(['tmux', 'split-window', '-Z',
                \ '-e', 'VIM_SERVERNAME=' . v:servername,
                \ l:tui_cmd])
endfunction

" ---------------------------------------------------------------------------
" patto_preview_toggle#schedule_restore(topline, lnum)
" Called via --remote-expr from the TUI's editor command.
" Restores the viewport after the next VimResized event (fired when tmux
" un-zooms the pane and the terminal is resized).
" ---------------------------------------------------------------------------
function! patto_preview_toggle#schedule_restore(topline, lnum) abort
    " Store pending restore info in script-local variables; the autocmd
    " fires once and then removes itself.
    let s:_restore_topline = a:topline
    let s:_restore_lnum    = a:lnum

    augroup patto_preview_toggle_restore
        au!
        au VimResized * call s:do_restore() | autocmd! patto_preview_toggle_restore
    augroup END

    " --remote-expr requires a non-empty string return value
    return ''
endfunction

" ---------------------------------------------------------------------------
" Internal: apply winrestview while keeping scrolloff from shifting the view.
" ---------------------------------------------------------------------------
function! s:do_restore() abort
    let l:topline = get(s:, '_restore_topline', 1)
    let l:lnum    = get(s:, '_restore_lnum',    1)
    let l:so      = &scrolloff
    let l:siso    = &sidescrolloff

    " Clamp lnum so it is never above topline (scrolloff would shift view)
    let l:safe_lnum = max([l:lnum, l:topline + l:so])
    let l:safe_lnum = min([l:safe_lnum, line('$')])

    let &scrolloff     = 0
    let &sidescrolloff = 0
    call winrestview({'topline': l:topline, 'lnum': l:safe_lnum})
    let &scrolloff     = l:so
    let &sidescrolloff = l:siso
endfunction
