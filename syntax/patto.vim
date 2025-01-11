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

"syn region  pattoSLink        keepend start=/\[/ms=s+1 end=/\]/me=e-1 contains=@pattoSBracketLink oneline transparent contained
syn region  pattoSBracket        keepend start=/\[/ms=s+1 end=/\]/me=e-1 contains=@pattoSBracketLink oneline
syn match pattoSBracketNoURL /\[\(.\+:\/\/\\*\)\@!.\{-}\]/ms=s+1,me=e-1 keepend contains=@pattoSBracketContent,pattoPageLink

" [patto]
" do not match url!
" exe 'syn match  pattoPageLink /\(.\{1,}:\/\/\S\{1,}\)\@!.\+/    contained'
" exe 'syn match pattoPageLink /\(.\{1,}:\/\/.\*\)\@!.\{-}/ contained'
"syn match pattoPageLink /^\(\(.\+:\/\/\\*\)\@!.\*\)$/ contained
"syn match pattoPageLink /.\+/ contained
syn match pattoPageLink /[^\[\]]\+/ contained  " not sure why I need to exlude '['

" [-*/_ patto]
syn match  pattoBody     /\s\{1,}[^\[\]]\+/ contained contains=@pattoSBracket transparent
"syn match  pattoBody     /\s\{1,}.\+/ contained contains=@pattoBracket0,@pattoBracket1,@pattoBracket2,@pattoBracket3,@pattoBracket4,@pattoBracket5,@pattoBracket6,@pattoBracket7,@pattoBracket8,@pattoBracket9 transparent
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
let url_regex = '\w\{1,}:\/\/\S\{1,}'
execute 'syn match  pattoSLink1  /\zs' . url_regex . '\ze/        contained'
" [url url_title]
execute 'syn match  pattoSLink2  /\zs\s*' . url_regex . '\s\{1,}\ze.\{1,}/ contained conceal cchar=🔗'
" [url_title url]
execute 'syn match  pattoSLink3   /.\{1,}\zs\s\{1,}' . url_regex . '\ze/ contained conceal cchar=🔗'

" [@img patto]
syn match  pattoSImg    /\[\zs@img\s\{1,}.*\ze\]/

" {@line_property ...}
syn region pattoLineProperty   start=/{@\w\+/ end=/}/ oneline
" #line_anchor
syn match  pattoLineAnchor   /.*\s\+\zs\#\S\+\ze$/
"syn match  pattoTag      /#\S\{1,}/
" some task {@task status=done}
syn match  pattoTaskHighPriority     /^\s*\zs.*{@task.*priority=high.*}.*$/
syn match  pattoTaskDone     /^\s*\zs.*{@task.*status=done.*}.*$/
" some task !date
syn match  pattoAbbrevTask   /.*\zs[!\*]\d\{4}\-\d\{2}\-\d\{2}\%[T\d\d\:\d\d}]\ze.*$/
syn match  pattoAbbrevTaskDone   /^\s*\zs.*\-\d\{4}\-\d\{2}\-\d\{2}\%[T\d\d\:\d\d}]\ze.*$/

""" Code
" [`"patto"`]
syn region pattoInlineCode     start=/\[`/ end=/`\]/ skip=/\\`/ oneline
" $ ./patto.sh or % ./patto.sh
"syn region pattoCode     start=/^\s*\$/ start=/^\s*%/ end=/$/
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
"hi def link pattoTag      Underlined
hi def link pattoBig      Type
hi def link pattoItalic   Keyword
hi def link pattoUnder    Underlined
hi def link pattoInlineMath    Operator
hi def link pattoNumber   Type
hi def link pattoInlineCode     String
hi def link pattoLineProperty  Comment
hi def link pattoLineAnchor Keyword
hi def link pattoCode     String
"hi def link pattoMath     Operator
hi def link pattoQuote    SpecialComment
hi def link pattoStrike   Comment
hi def link pattoTaskHighPriority Type
hi def link pattoTaskDone NonText
hi def link pattoAbbrevTask Type
hi def link pattoAbbrevTaskDone NonText
hi Folded ctermbg=Black ctermfg=Yellow


"--------------------------
"hi def hlLevel0 ctermfg=red		guifg=red1
"hi def hlLevel1 ctermfg=yellow	guifg=orange1
"hi def hlLevel2 ctermfg=green	guifg=yellow1
"hi def hlLevel3 ctermfg=cyan	guifg=greenyellow
"hi def hlLevel4 ctermfg=magenta	guifg=green1
"hi def hlLevel5 ctermfg=red		guifg=springgreen1
"hi def hlLevel6 ctermfg=yellow	guifg=cyan1
"hi def hlLevel7 ctermfg=green	guifg=slateblue1
"hi def hlLevel8 ctermfg=cyan	guifg=magenta1
"hi def hlLevel9 ctermfg=magenta	guifg=purple1
" syn region pattoBracket0           matchgroup=hlLevel0 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket1
" syn region pattoBracket1 contained matchgroup=hlLevel1 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket2
" syn region pattoBracket2 contained matchgroup=hlLevel2 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket3
" syn region pattoBracket3 contained matchgroup=hlLevel3 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket4
" syn region pattoBracket4 contained matchgroup=hlLevel4 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket5
" syn region pattoBracket5 contained matchgroup=hlLevel5 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket6
" syn region pattoBracket6 contained matchgroup=hlLevel6 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket7
" syn region pattoBracket7 contained matchgroup=hlLevel7 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket8
" syn region pattoBracket8 contained matchgroup=hlLevel8 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket9
" syn region pattoBracket9 contained matchgroup=hlLevel9 start="\[" end="\]" skip="|.\{-}|" contains=@pattoSBracketLink,pattoBracket0

