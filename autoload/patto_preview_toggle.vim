" Vim port of lua/patto_preview_toggle.lua.
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

    " $VIM_SERVERNAME lets the TUI's editor command reach this Vim instance
    " via --remote / --remote-expr.
    call system(['tmux', 'split-window', '-Z',
                \ '-e', 'VIM_SERVERNAME=' . v:servername,
                \ l:tui_cmd])
endfunction

" Called via --remote-expr from the TUI's editor command. The tmux unzoom
" resizes the terminal, so the restore runs on the next VimResized.
function! patto_preview_toggle#schedule_restore(topline, lnum) abort
    let s:_restore_topline = a:topline
    let s:_restore_lnum    = a:lnum

    augroup patto_preview_toggle_restore
        au!
        au VimResized * call s:do_restore() | autocmd! patto_preview_toggle_restore
    augroup END

    return ''
endfunction

function! s:do_restore() abort
    let l:topline = get(s:, '_restore_topline', 1)
    let l:lnum    = get(s:, '_restore_lnum',    1)
    let l:so      = &scrolloff
    let l:siso    = &sidescrolloff

    " scrolloff would move topline while the cursor is placed, so lnum is
    " clamped below it and scrolloff is turned off around winrestview.
    let l:safe_lnum = max([l:lnum, l:topline + l:so])
    let l:safe_lnum = min([l:safe_lnum, line('$')])

    let &scrolloff     = 0
    let &sidescrolloff = 0
    call winrestview({'topline': l:topline, 'lnum': l:safe_lnum})
    let &scrolloff     = l:so
    let &sidescrolloff = l:siso
endfunction
