" originally taken from:
" https://github.com/syusui-s/scrapbox-vim/blob/master/syntax/scrapbox.vim

"  Original Copyright:
"  Scrapbox Syntax Plugin
"  Maintainer: Syusui Moyatani <syusui.s[a]gmail.com>
"  License: Creative Commons Zero 1.0 Universal
"  Version: 1.0.0

syn clear

""" Brackets
syn cluster tabtonSBracketContent contains=tabtonBig,tabtonItalic,tabtonStrike,tabtonUnder,tabtonBody,tabtonInlineMath
syn cluster tabtonSBracketLink    contains=tabtonSLink1,tabtonSLink2,tabtonSLink3

"syn region  tabtonSLink        keepend start=/\[/ms=s+1 end=/\]/me=e-1 contains=@tabtonSBracketLink oneline transparent contained
syn region  tabtonSBracket        keepend start=/\[/ms=s+1 end=/\]/me=e-1 contains=@tabtonSBracketLink oneline
syn match tabtonSBracketNoURL /\[\(.\+:\/\/\\*\)\@!.\{-}\]/ms=s+1,me=e-1 keepend contains=@tabtonSBracketContent,tabtonPageLink

" [tabton]
" do not match url!
" exe 'syn match  tabtonPageLink /\(.\{1,}:\/\/\S\{1,}\)\@!.\+/    contained'
" exe 'syn match tabtonPageLink /\(.\{1,}:\/\/.\*\)\@!.\{-}/ contained'
"syn match tabtonPageLink /^\(\(.\+:\/\/\\*\)\@!.\*\)$/ contained
"syn match tabtonPageLink /.\+/ contained
syn match tabtonPageLink /[^\[\]]\+/ contained  " not sure why I need to exlude '['

" [-*/_ tabton]
syn match  tabtonBody     /\s\{1,}[^\[\]]\+/ contained contains=@tabtonSBracket transparent
"syn match  tabtonBody     /\s\{1,}.\+/ contained contains=@tabtonBracket0,@tabtonBracket1,@tabtonBracket2,@tabtonBracket3,@tabtonBracket4,@tabtonBracket5,@tabtonBracket6,@tabtonBracket7,@tabtonBracket8,@tabtonBracket9 transparent
" [- tabton]
syn match  tabtonStrike   /-\{1,}[^\[\]]\+/  contained contains=@tabtonSBracketContent
" [/ tabton]
syn match  tabtonItalic   /\/\{1,}[^\[\]]\+/ contained contains=@tabtonSBracketContent
" [* tabton]
syn match  tabtonBig      /\*\{1,}[^\[\]]\+/ contained contains=@tabtonSBracketContent
" [_ tabton]
syn match  tabtonUnder    /_\{1,}[^\[\]]\+/  contained contains=@tabtonSBracketContent

" [$ tabton$]
syn include @tex syntax/tex.vim
syn region tabtonInlineMath start="\\\@<!\$" end="\$" skip="\\\$" contained contains=@tex keepend

" [url]
let url_regex = '\w\{1,}:\/\/\S\{1,}'
execute 'syn match  tabtonSLink1  /\zs' . url_regex . '\ze/        contained'
" [url url_title]
execute 'syn match  tabtonSLink2  /\zs\s*' . url_regex . '\s\{1,}\ze.\{1,}/ contained conceal cchar=🔗'
" [url_title url]
execute 'syn match  tabtonSLink3   /.\{1,}\zs\s\{1,}' . url_regex . '\ze/ contained conceal cchar=🔗'

" [@img tabton]
syn match  tabtonSImg    /\[\zs@img\s\{1,}.*\ze\]/

" {@line_property ...}
syn region tabtonLineProperty   start=/{@\w\+/ end=/}/ oneline
" #line_anchor
syn match  tabtonLineAnchor   /.*\s\+\zs\#\S\+\ze$/
"syn match  tabtonTag      /#\S\{1,}/
" some task {@task status=done}
syn match  tabtonTaskHighPriority     /^\s*\zs.*{@task.*priority=high.*}.*$/
syn match  tabtonTaskDone     /^\s*\zs.*{@task.*status=done.*}.*$/

""" Code
" [`"tabton"`]
syn region tabtonInlineCode     start=/\[`/ end=/`\]/ skip=/\\`/ oneline
" $ ./tabton.sh or % ./tabton.sh
"syn region tabtonCode     start=/^\s*\$/ start=/^\s*%/ end=/$/
" [@code lang]
syn region tabtonCode start=/^\z(\s*\)\[@code \(\S\+\)\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/
" [@math]
syn region tabtonMath     matchgroup=texDelimiter start=/^\z(\s*\)\[@math\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/ contains=@texMathZoneGroup keepend
" [@quote]
syn region tabtonQuote     start=/^\z(\s*\)\[@quote\]/ skip=/^\(\z1\s\|\n\+\z1\)/ end=/^/

""" Highlight

hi def link tabtonTitle    Function
hi def link tabtonPageLink Structure
hi def link tabtonSImg Type
hi def link tabtonSBracket Operator
hi def link tabtonSLink1   Operator
hi def link tabtonSLink2   Operator
hi def link tabtonSLink3   Operator
"hi def link tabtonTag      Underlined
hi def link tabtonBig      Type
hi def link tabtonItalic   Keyword
hi def link tabtonUnder    Underlined
hi def link tabtonInlineMath    Operator
hi def link tabtonNumber   Type
hi def link tabtonInlineCode     String
hi def link tabtonLineProperty  Comment
hi def link tabtonLineAnchor Keyword
hi def link tabtonCode     String
"hi def link tabtonMath     Operator
hi def link tabtonQuote    SpecialComment
hi def link tabtonStrike   Comment
hi def link tabtonTaskHighPriority Type
hi def link tabtonTaskDone NonText
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
" syn region tabtonBracket0           matchgroup=hlLevel0 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket1
" syn region tabtonBracket1 contained matchgroup=hlLevel1 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket2
" syn region tabtonBracket2 contained matchgroup=hlLevel2 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket3
" syn region tabtonBracket3 contained matchgroup=hlLevel3 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket4
" syn region tabtonBracket4 contained matchgroup=hlLevel4 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket5
" syn region tabtonBracket5 contained matchgroup=hlLevel5 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket6
" syn region tabtonBracket6 contained matchgroup=hlLevel6 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket7
" syn region tabtonBracket7 contained matchgroup=hlLevel7 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket8
" syn region tabtonBracket8 contained matchgroup=hlLevel8 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket9
" syn region tabtonBracket9 contained matchgroup=hlLevel9 start="\[" end="\]" skip="|.\{-}|" contains=@tabtonSBracketLink,tabtonBracket0

