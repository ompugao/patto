" originally taken from:
" https://github.com/syusui-s/scrapbox-vim/blob/master/syntax/scrapbox.vim

"  Original Copyright:
"  Scrapbox Syntax Plugin
"  Maintainer: Syusui Moyatani <syusui.s[a]gmail.com>
"  License: Creative Commons Zero 1.0 Universal
"  Version: 1.0.0

syn clear

""" Brackets
syn cluster pattoSBracketContent contains=pattoBig,pattoItalic,pattoStrike,pattoUnder,pattoBody,pattoInlineMath
syn cluster pattoSBracketLink    contains=pattoSLink1,pattoSLink2,pattoSLink3

syn region  pattoSBracket        keepend start=/\[/ms=s+1 end=/\]/me=e-1 contains=@pattoSBracketLink oneline
syn match pattoSBracketNoURL /\[\(.\+:\/\/\)\@!.\{-}\]/ms=s+1,me=e-1 keepend contains=@pattoSBracketContent,pattoPageLink

" [patto]
" do not match url!
syn match pattoPageLink /[^\[\]]\+/ contained  " not sure why I need to exlude '['

" [-*/_ patto]
syn match  pattoBody     /\s\{1,}[^\[\]]\+/ contained contains=@pattoSBracket transparent
" [- patto]
syn match  pattoStrike   /-\{1,}[^\[\]]\+/  contained contains=@pattoSBracketContent
" [/ patto]
syn match  pattoItalic   /\/\{1,}[^\[\]]\+/ contained contains=@pattoSBracketContent
" [* patto]
syn match  pattoBig      /\*\{1,}[^\[\]]\+/ contained contains=@pattoSBracketContent
" [_ patto]
syn match  pattoUnder    /_\{1,}[^\[\]]\+/  contained contains=@pattoSBracketContent

" [$ patto$]
syn include @tex syntax/tex.vim
syn region pattoInlineMath start="\\\@<!\$" end="\$" skip="\\\$" contained contains=@tex keepend

" [url]
let url_regex = '\w\{1,}:\/\/[^\] \t]\{1,}'
execute 'syn match  pattoSLink1  /\zs' . url_regex . '\ze/        contained'
" [url url_title]
execute 'syn match  pattoSLink2  /\zs\s*' . url_regex . '\s\{1,}\ze.\{1,}/ contained conceal cchar=🔗'
" [url_title url]
execute 'syn match  pattoSLink3   /.\{1,}\zs\s\{1,}' . url_regex . '\ze/ contained conceal cchar=🔗'

" [@img patto]
syn match  pattoSImg    /\[\zs@img\s\{1,}.*\ze\]/

" {@line_property ...}
syn region pattoLineProperty   start=/{@\w\+/ end=/}/ oneline

" {@task} concealment
syn match pattoTaskBrace      /[{}]/                          contained conceal
syn match pattoTaskAt         /@task\s*/                      contained conceal
syn match pattoTaskPropStatus /status=/                       contained conceal cchar=◆
syn match pattoTaskPropDue    /due=/                          contained conceal cchar=⏰
syn match pattoTaskPropHidden /\(status\|due\)\@!\w\+=\S\+/  contained conceal
syn region pattoTaskProperty  start=/{@task/ end=/}/ oneline
  \ contains=pattoTaskBrace,pattoTaskAt,pattoTaskPropStatus,pattoTaskPropDue,pattoTaskPropHidden
" #line_anchor
syn match  pattoLineAnchor   /.*\s\+\zs\#\S\+\ze$/
" transparent bracket wrapper for use inside done/high-priority task lines
syn region pattoTaskBracket keepend start=/\[/ end=/\]/ oneline contained transparent contains=pattoSLink2,pattoSLink3

" some task {@task status=done}
syn match  pattoTaskHighPriority     /^\s*\zs.*{@task.*priority=high.*}.*$/ contains=pattoTaskProperty,pattoTaskBracket
syn match  pattoTaskDone     /^\s*\zs.*{@task.*status=done.*}.*$/ contains=pattoTaskProperty,pattoTaskBracket
" some task !date
syn match  pattoAbbrevTask   /.*\zs[!\*]\d\{4}\-\d\{2}\-\d\{2}\%[T\d\d\:\d\d}]\ze.*$/
syn match  pattoAbbrevTaskDone   /^\s*\zs.*\-\d\{4}\-\d\{2}\-\d\{2}\%[T\d\d\:\d\d}]\ze.*$/

""" Code
" [`"patto"`]
syn region pattoInlineCode     start=/\[`/ end=/`\]/ skip=/\\`/ oneline
" [@code lang]
syn region pattoCode start=/^\z(\s*\)\[@code \(\S\+\)\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/
" [@math]
syn region pattoMath     matchgroup=texDelimiter start=/^\z(\s*\)\[@math\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/ contains=@texMathZoneGroup keepend
" [@quote]
syn region pattoQuote     start=/^\z(\s*\)\[@quote\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/

""" Highlight

hi def link pattoTitle    Function
hi def link pattoPageLink Structure
hi def link pattoSImg Type
hi def link pattoSBracket Operator
hi def link pattoSLink1   Operator
hi def link pattoSLink2   Operator
hi def link pattoSLink3   Operator
hi def link pattoBig      Type
hi def link pattoItalic   Keyword
hi def link pattoUnder    Underlined
hi def link pattoInlineMath    Operator
hi def link pattoNumber   Type
hi def link pattoInlineCode     String
hi def link pattoLineProperty  Comment
hi def link pattoTaskProperty  Comment
hi def link pattoLineAnchor Keyword
hi def link pattoCode     String
hi def link pattoQuote    SpecialComment
hi def link pattoStrike   Comment
hi def link pattoTaskHighPriority Type
hi def link pattoTaskDone NonText
hi def link pattoAbbrevTask Type
hi def link pattoAbbrevTaskDone NonText
hi Folded ctermbg=Black ctermfg=Yellow
