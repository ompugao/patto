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
endfunction

function! s:on_lsp_buffer_enabled() abort
  command! -buffer LspPattoTasks call <SID>patto_tasks()
  nnoremap <buffer> <plug>(lsp-patto-tasks) :<c-u>call <SID>patto_tasks()<cr>
  command! -buffer LspPattoScanWorkspace call <SID>patto_scan_workspace()
  nnoremap <buffer> <plug>(lsp-patto-scan-workspace) :<c-u>call <SID>patto_scan_workspace()<cr>
endfunction

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
        \   'next':{x->s:show_task(x['response']['result'])},
        \   'error':{e->lsp_settings#utils#error(e)},
        \ })
        \ )
endfunction

function! s:show_task(res) abort
    let l:list = []
    for l:item in a:res
        let l:path = lsp#utils#uri_to_path(l:item['location']['uri'])
        let [l:line, l:col] = lsp#utils#position#lsp_to_vim(l:path, l:item['location']['range']['start'])
        let l:location_item = {
            \ 'filename': l:path,
            \ 'lnum': l:line,
            \ 'col': l:col,
            \ 'text': l:item['text']
            \ }
        call add(l:list, l:location_item)
    endfor

    if empty(l:list)
        call lsp#utils#error('No tasks. Great!')
        return
    else
        call setloclist(0, l:list)
        echo 'Retrieved tasks'
        botright lopen 8
    endif
endfunction


function! s:patto_scan_workspace() abort
    call lsp#callbag#pipe(
        \ lsp#request('patto-lsp', {
        \   'method': 'workspace/executeCommand',
        \   'params': {
        \       'command': 'experimental/scan_workspace',
        \       'arguments': [],
        \   }
        \ }),
        \ )
endfunction
