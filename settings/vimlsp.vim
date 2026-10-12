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

function! s:is_port_open(host, port) abort
    if executable('nc')
        call system('nc -z -w1 ' . shellescape(a:host) . ' ' . a:port . ' 2>/dev/null')
        return v:shell_error == 0
    endif
    call system('bash -c "echo > /dev/tcp/' . a:host . '/' . a:port . '" 2>/dev/null')
    return v:shell_error == 0
endfunction

function! s:on_lsp_buffer_enabled() abort
    " LSP-based folding; enable with: let g:patto_lsp_folding = 1
    if get(g:, 'patto_lsp_folding', 0)
        setlocal foldmethod=expr
        setlocal foldexpr=lsp#ui#vim#folding#foldexpr()
        setlocal foldlevel=99
    endif

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

function! s:execute_command(command, arguments, next) abort
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': a:command,
        \       'arguments': a:arguments,
        \   }
        \ }),
        \ lsp#callbag#subscribe({
        \   'next':  a:next,
        \   'error': {e -> lsp#utils#error(string(e))},
        \ })
        \ )
endfunction

function! s:time_spent_chip(ts) abort
    if type(a:ts) != v:t_dict
        return ''
    endif
    let l:h = get(a:ts, 'hours', 0)
    let l:m = get(a:ts, 'minutes', 0)
    if l:h > 0 && l:m > 0
        return '[' . l:h . 'h' . l:m . 'm]'
    elseif l:h > 0
        return '[' . l:h . 'h]'
    elseif l:m > 0
        return '[' . l:m . 'm]'
    endif
    return ''
endfunction

function! s:loclist_entry(task, parts) abort
    let l:path = lsp#utils#uri_to_path(a:task['location']['uri'])
    let [l:line, l:col] = lsp#utils#position#lsp_to_vim(l:path, a:task['location']['range']['start'])
    return {
                \ 'filename': l:path,
                \ 'lnum':     l:line,
                \ 'col':      l:col,
                \ 'text':     join(a:parts, ' '),
                \ }
endfunction

" :LspPattoTasks
function! s:patto_tasks() abort
    call s:execute_command('experimental/aggregate_tasks', [],
                \ {x -> s:show_task(x['response']['result'])})
endfunction

function! s:task_parts(item) abort
    let l:parts = []

    let l:due = get(a:item, 'due', v:null)
    if type(l:due) == v:t_dict
        let l:due_str = get(l:due, 'Date', get(l:due, 'DateTime', ''))
        if l:due_str !=# ''
            let l:due_str = substitute(l:due_str, 'T.*$', '', '')
            call add(l:parts, '[due:' . l:due_str . ']')
        endif
    endif

    call add(l:parts, a:item['text'])

    let l:status = get(a:item, 'status', '')
    if l:status ==# 'Doing'
        call add(l:parts, '[doing]')
    elseif l:status ==# 'Paused'
        call add(l:parts, '[paused]')
    endif

    let l:chip = s:time_spent_chip(get(a:item, 'time_spent', v:null))
    if l:chip !=# ''
        call add(l:parts, l:chip)
    endif
    return l:parts
endfunction

function! s:show_task(res) abort
    let l:list = []
    for l:item in a:res
        call add(l:list, s:loclist_entry(l:item, s:task_parts(l:item)))
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

" :LspPattoScanWorkspace
function! s:patto_scan_workspace() abort
    call s:execute_command('experimental/scan_workspace', [],
                \ {x -> execute('echomsg "patto: workspace scanned"', '')})
endfunction

" :LspPattoSnapshotPapers
function! s:patto_snapshot_papers() abort
    call s:execute_command('patto/snapshotPapers', [],
                \ {x -> execute('echomsg "patto: papers snapshotted"', '')})
endfunction

" :LspPattoTwoHopLinks
function! s:patto_two_hop_links() abort
    let l:uri = lsp#utils#path_to_uri(expand('%:p'))
    call s:execute_command('experimental/retrieve_two_hop_notes', [l:uri],
                \ {x -> s:show_two_hop_links(x['response']['result'])})
endfunction

function! s:show_two_hop_links(result) abort
    if empty(a:result)
        echomsg 'patto: No 2-hop links'
        return
    endif

    let l:lines = []
    let l:paths = []
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

    let b:patto_two_hop_paths = l:paths
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

" :LspPattoTasksReview [today|yesterday|this_week|last_week|this_month|FROM:TO]
function! s:tasks_review_complete(arglead, cmdline, cursorpos) abort
    return filter(['today','yesterday','this_week','last_week','this_month'],
                \ 'v:val =~ "^" . a:arglead')
endfunction

function! s:patto_tasks_review(arg) abort
    let l:arg = a:arg !=# '' ? a:arg : 'today'
    let l:named = ['today', 'yesterday', 'this_week', 'last_week', 'this_month']

    if index(l:named, l:arg) >= 0
        let l:arguments = [l:arg]
    else
        let l:m = matchlist(l:arg, '^\(\d\{4}-\d\{2}-\d\{2}\):\(\d\{4}-\d\{2}-\d\{2}\)$')
        if empty(l:m)
            call lsp#utils#error('LspPattoTasksReview: invalid argument "' . l:arg
                        \ . '". Use today|yesterday|this_week|last_week|this_month|YYYY-MM-DD:YYYY-MM-DD')
            return
        endif
        let l:arguments = ['custom', l:m[1], l:m[2]]
    endif

    call s:execute_command('experimental/tasks_review', l:arguments,
                \ {x -> s:show_tasks_review(x['response']['result'], l:arg)})
endfunction

function! s:review_parts(task) abort
    let l:parts = []
    let l:cat = get(a:task, 'completed_at', '')
    if type(l:cat) == v:t_string && l:cat !=# ''
        call add(l:parts, '[' . l:cat . ']')
    endif
    call add(l:parts, a:task['text'])
    let l:chip = s:time_spent_chip(get(a:task, 'time_spent', v:null))
    if l:chip !=# ''
        call add(l:parts, l:chip)
    endif
    return l:parts
endfunction

function! s:show_tasks_review(res, label) abort
    if empty(a:res)
        echomsg 'patto: No completed tasks found for: ' . a:label
        return
    endif

    let l:list = []
    for l:task in a:res
        call add(l:list, s:loclist_entry(l:task, s:review_parts(l:task)))
    endfor

    call setloclist(0, l:list)
    echomsg 'patto: ' . len(l:list) . ' completed task(s) for: ' . a:label
    botright lopen 10
    setlocal nowrap
endfunction

" :LspPattoCopyAsMarkdown [flavor]   (works with ranges / visual selection)
function! s:markdown_flavor_complete(arglead, cmdline, cursorpos) abort
    return filter(['standard','obsidian','github'],
                \ 'v:val =~ "^" . a:arglead')
endfunction

function! s:patto_copy_as_markdown(flavor_arg, range, line1, line2) abort
    let l:uri    = lsp#utils#path_to_uri(expand('%:p'))
    let l:flavor = a:flavor_arg !=# '' ? a:flavor_arg : v:null

    if a:range == 2
        " LSP lines are 0-indexed
        let l:args = [l:uri, a:line1 - 1, a:line2 - 1, l:flavor]
    else
        let l:args = [l:uri, v:null, v:null, l:flavor]
    endif

    call s:execute_command('patto/renderAsMarkdown', l:args,
                \ {x -> s:yank_markdown(x['response']['result'], a:flavor_arg)})
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
