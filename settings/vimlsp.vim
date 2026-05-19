augroup patto-language-server
    au!
    au User lsp_setup call s:setup_server()
    au User lsp_buffer_enabled call s:on_lsp_buffer_enabled()
augroup END

function! s:setup_server() abort
    let s:patto_client_id = lsp#register_server({
                \ 'name': 'patto-lsp',
                \ 'cmd': ['patto-lsp' ],
                \ 'allowlist': ['patto'],
                \ })
    let s:patto_preview_client_id = lsp#register_server({
                \ 'name': 'patto-preview',
                \ 'cmd': ['patto-preview', '--preview-lsp-stdio'],
                \ 'allowlist': ['patto'],
                \ })
    " patto-preview-tui: connect via TCP (port g:patto_preview_tui_port, default 9527).
    " The TUI must already be running before opening a .pn file.
    let l:tui_port = get(g:, 'patto_preview_tui_port', 9527)
    if s:is_port_open('127.0.0.1', l:tui_port)
        let s:patto_preview_tui_client_id = lsp#register_server({
                    \ 'name': 'patto-preview-tui',
                    \ 'tcp': '127.0.0.1:' . l:tui_port,
                    \ 'allowlist': ['patto'],
                    \ })
    else
        echomsg 'patto-preview-tui is not running. Start it first (port ' . l:tui_port . ')'
    endif
endfunction

" Check whether a TCP port is open by attempting a connection with nc/curl.
function! s:is_port_open(host, port) abort
    " Try nc (netcat) first, then fall back to /dev/tcp
    if executable('nc')
        let l:ret = system('nc -z -w1 ' . shellescape(a:host) . ' ' . a:port . ' 2>/dev/null')
        return v:shell_error == 0
    endif
    " bash /dev/tcp fallback
    let l:ret = system('bash -c "echo > /dev/tcp/' . a:host . '/' . a:port . '" 2>/dev/null')
    return v:shell_error == 0
endfunction

function! s:on_lsp_buffer_enabled() abort
    command! -buffer LspPattoTasks          call <SID>patto_tasks()
    command! -buffer LspPattoScanWorkspace  call <SID>patto_scan_workspace()
    command! -buffer LspPattoSnapshotPapers call <SID>patto_snapshot_papers()
    command! -buffer LspPattoTwoHopLinks    call <SID>patto_two_hop_links()
    command! -buffer -nargs=? -complete=customlist,<SID>tasks_review_complete
                \ LspPattoTasksReview       call <SID>patto_tasks_review(<q-args>)
    command! -buffer -range -nargs=? -complete=customlist,<SID>markdown_flavor_complete
                \ LspPattoCopyAsMarkdown    call <SID>patto_copy_as_markdown(<q-args>, <range>, <line1>, <line2>)

    nnoremap <buffer> <plug>(lsp-patto-tasks)
                \ :<c-u>call <SID>patto_tasks()<cr>
    nnoremap <buffer> <plug>(lsp-patto-scan-workspace)
                \ :<c-u>call <SID>patto_scan_workspace()<cr>
    nnoremap <buffer> <plug>(lsp-patto-two-hop-links)
                \ :<c-u>call <SID>patto_two_hop_links()<cr>
endfunction

" ---------------------------------------------------------------------------
" :LspPattoTasks — aggregate pending tasks into the location list
" ---------------------------------------------------------------------------
function! s:patto_tasks() abort
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'experimental/aggregate_tasks',
        \       'arguments': [],
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> s:show_task(x['response']['result'])},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

function! s:show_task(res) abort
    let l:list = []
    for l:item in a:res
        let l:path = lsp#utils#uri_to_path(l:item['location']['uri'])
        let [l:line, l:col] = lsp#utils#position#lsp_to_vim(l:path, l:item['location']['range']['start'])

        " Build a rich display string: due date + label + chips
        let l:parts = []

        " due date chip first
        let l:due = get(l:item, 'due', v:null)
        if type(l:due) == v:t_dict
            let l:due_str = get(l:due, 'Date', get(l:due, 'DateTime', ''))
            if l:due_str !=# ''
                " Trim datetime to date portion
                let l:due_str = substitute(l:due_str, 'T.*$', '', '')
                call add(l:parts, '[due:' . l:due_str . ']')
            endif
        endif

        call add(l:parts, l:item['text'])

        " status chip (only show non-todo)
        let l:status = get(l:item, 'status', '')
        if l:status ==# 'Doing'
            call add(l:parts, '[doing]')
        elseif l:status ==# 'Paused'
            call add(l:parts, '[paused]')
        endif

        " time_spent chip
        let l:ts = get(l:item, 'time_spent', v:null)
        if type(l:ts) == v:t_dict
            let l:h = get(l:ts, 'hours', 0)
            let l:m = get(l:ts, 'minutes', 0)
            if l:h > 0 && l:m > 0
                call add(l:parts, '[' . l:h . 'h' . l:m . 'm]')
            elseif l:h > 0
                call add(l:parts, '[' . l:h . 'h]')
            elseif l:m > 0
                call add(l:parts, '[' . l:m . 'm]')
            endif
        endif

        call add(l:list, {
                    \ 'filename': l:path,
                    \ 'lnum':     l:line,
                    \ 'col':      l:col,
                    \ 'text':     join(l:parts, ' '),
                    \ })
    endfor

    if empty(l:list)
        call lsp#utils#error('No tasks. Great!')
        return
    endif
    call setloclist(0, l:list)
    echo 'Retrieved tasks'
    botright lopen 8
    setlocal nowrap
endfunction

" ---------------------------------------------------------------------------
" :LspPattoScanWorkspace
" ---------------------------------------------------------------------------
function! s:patto_scan_workspace() abort
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'experimental/scan_workspace',
        \       'arguments': [],
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> execute('echomsg "patto: workspace scanned"', '')},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

" ---------------------------------------------------------------------------
" :LspPattoSnapshotPapers
" ---------------------------------------------------------------------------
function! s:patto_snapshot_papers() abort
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'patto/snapshotPapers',
        \       'arguments': [],
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> execute('echomsg "patto: papers snapshotted"', '')},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

" ---------------------------------------------------------------------------
" :LspPattoTwoHopLinks — show 2-hop links in a scratch buffer
" ---------------------------------------------------------------------------
function! s:patto_two_hop_links() abort
    let l:uri = lsp#utils#path_to_uri(expand('%:p'))
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'experimental/retrieve_two_hop_notes',
        \       'arguments': [l:uri],
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> s:show_two_hop_links(x['response']['result'])},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

function! s:show_two_hop_links(result) abort
    if empty(a:result)
        echomsg 'patto: No 2-hop links'
        return
    endif

    " Build display lines and a parallel list of file paths
    let l:lines = []
    let l:paths = []   " parallel list: path for each line (or '' for headers)
    for l:group in a:result
        let l:nearest_uri  = l:group[0]
        let l:two_hop_uris = l:group[1]
        let l:nearest_path = lsp#utils#uri_to_path(l:nearest_uri)
        let l:nearest_name = fnamemodify(l:nearest_path, ':t')
        call add(l:lines, l:nearest_name . '  [' . l:nearest_path . ']')
        call add(l:paths, l:nearest_path)
        for l:link_uri in l:two_hop_uris
            let l:link_path = lsp#utils#uri_to_path(l:link_uri)
            let l:link_name = fnamemodify(l:link_path, ':t')
            call add(l:lines, '  - ' . l:link_name . '  [' . l:link_path . ']')
            call add(l:paths, l:link_path)
        endfor
    endfor

    " Open / reuse a scratch buffer
    let l:bufname = 'patto://[2hop links]'
    let l:bufnr = bufnr(l:bufname)
    if l:bufnr == -1
        let l:height = max([5, winheight(0) / 3])
        execute 'botright ' . l:height . 'split ' . fnameescape(l:bufname)
        setlocal buftype=nofile bufhidden=wipe noswapfile nobuflisted
        setlocal nowrap nonumber norelativenumber nospell
    else
        let l:winid = bufwinid(l:bufnr)
        if l:winid != -1
            call win_gotoid(l:winid)
        else
            let l:height = max([5, winheight(0) / 3])
            execute 'botright ' . l:height . 'split +buffer\ ' . l:bufnr
        endif
    endif

    setlocal modifiable
    silent %delete _
    call setline(1, l:lines)
    setlocal nomodifiable nomodified

    " Store the path list as a buffer-local variable for <CR> mapping
    let b:patto_two_hop_paths = l:paths

    " <CR>: open the file whose path is embedded in the current line
    nnoremap <buffer> <silent> <CR> :<C-u>call <SID>two_hop_open_under_cursor()<CR>
endfunction

function! s:two_hop_open_under_cursor() abort
    if !exists('b:patto_two_hop_paths')
        return
    endif
    let l:idx = line('.') - 1
    if l:idx < 0 || l:idx >= len(b:patto_two_hop_paths)
        return
    endif
    let l:path = b:patto_two_hop_paths[l:idx]
    if l:path !=# ''
        execute 'edit ' . fnameescape(l:path)
    endif
endfunction

" ---------------------------------------------------------------------------
" :LspPattoTasksReview [today|yesterday|this_week|last_week|this_month|FROM:TO]
" ---------------------------------------------------------------------------
function! s:tasks_review_complete(arglead, cmdline, cursorpos) abort
    return filter(['today','yesterday','this_week','last_week','this_month'],
                \ 'v:val =~ "^" . a:arglead')
endfunction

function! s:patto_tasks_review(arg) abort
    let l:arg = a:arg !=# '' ? a:arg : 'today'
    let l:named = ['today', 'yesterday', 'this_week', 'last_week', 'this_month']
    let l:arguments = []

    if index(l:named, l:arg) >= 0
        let l:arguments = [l:arg]
    else
        " Try YYYY-MM-DD:YYYY-MM-DD
        let l:m = matchlist(l:arg, '^\(\d\{4}-\d\{2}-\d\{2}\):\(\d\{4}-\d\{2}-\d\{2}\)$')
        if !empty(l:m)
            let l:arguments = ['custom', l:m[1], l:m[2]]
        else
            call lsp#utils#error('LspPattoTasksReview: invalid argument "' . l:arg
                        \ . '". Use today|yesterday|this_week|last_week|this_month|YYYY-MM-DD:YYYY-MM-DD')
            return
        endif
    endif

    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'experimental/tasks_review',
        \       'arguments': l:arguments,
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> s:show_tasks_review(x['response']['result'], l:arg)},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

function! s:show_tasks_review(res, label) abort
    if empty(a:res)
        echomsg 'patto: No completed tasks found for: ' . a:label
        return
    endif

    let l:list = []
    for l:task in a:res
        let l:path = lsp#utils#uri_to_path(l:task['location']['uri'])
        let [l:line, l:col] = lsp#utils#position#lsp_to_vim(l:path, l:task['location']['range']['start'])

        " Build display: [completed_at] label [time_spent]
        let l:parts = []
        let l:cat = get(l:task, 'completed_at', '')
        if type(l:cat) == v:t_string && l:cat !=# ''
            call add(l:parts, '[' . l:cat . ']')
        endif
        call add(l:parts, l:task['text'])
        let l:ts = get(l:task, 'time_spent', v:null)
        if type(l:ts) == v:t_dict
            let l:h = get(l:ts, 'hours', 0)
            let l:m = get(l:ts, 'minutes', 0)
            if l:h > 0 && l:m > 0
                call add(l:parts, '[' . l:h . 'h' . l:m . 'm]')
            elseif l:h > 0
                call add(l:parts, '[' . l:h . 'h]')
            elseif l:m > 0
                call add(l:parts, '[' . l:m . 'm]')
            endif
        endif

        call add(l:list, {
                    \ 'filename': l:path,
                    \ 'lnum':     l:line,
                    \ 'col':      l:col,
                    \ 'text':     join(l:parts, ' '),
                    \ })
    endfor

    call setloclist(0, l:list)
    echomsg 'patto: ' . len(l:list) . ' completed task(s) for: ' . a:label
    botright lopen 10
    setlocal nowrap
endfunction

" ---------------------------------------------------------------------------
" :LspPattoCopyAsMarkdown [flavor]   (works with ranges / visual selection)
" ---------------------------------------------------------------------------
function! s:markdown_flavor_complete(arglead, cmdline, cursorpos) abort
    return filter(['standard','obsidian','github'],
                \ 'v:val =~ "^" . a:arglead')
endfunction

function! s:patto_copy_as_markdown(flavor_arg, range, line1, line2) abort
    let l:uri    = lsp#utils#path_to_uri(expand('%:p'))
    let l:flavor = a:flavor_arg !=# '' ? a:flavor_arg : v:null

    if a:range == 2
        " Convert Vim 1-indexed lines to 0-indexed for LSP
        let l:args = [l:uri, a:line1 - 1, a:line2 - 1, l:flavor]
    else
        let l:args = [l:uri, v:null, v:null, l:flavor]
    endif

    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'patto/renderAsMarkdown',
        \       'arguments': l:args,
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  {x -> s:yank_markdown(x['response']['result'], a:flavor_arg)},
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

function! s:yank_markdown(result, flavor_arg) abort
    if type(a:result) != type('') || a:result ==# ''
        call lsp#utils#error('patto: renderAsMarkdown returned no content')
        return
    endif
    call setreg('+', a:result)
    call setreg('"', a:result)
    let l:flavor = a:flavor_arg !=# '' ? a:flavor_arg : 'standard'
    echomsg 'patto: Copied as markdown (' . l:flavor . ')'
endfunction
