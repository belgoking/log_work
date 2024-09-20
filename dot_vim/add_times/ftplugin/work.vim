" This ftplugin works with files with "ft=work" you should copy it to "$VIMRUNTIME/ftplugin/work.vim".
" For Unix this is usually "~/.vim/ftplugin/work.vim"
" You can find out your "$VIMRUNTIME" by typing ":set runtimepath?" into vim.
"
" For this to work, you MUST add "filetype plugin on" to your ".vimrc" file

if exists("b:did_ftplugin")
  finish
endif
let b:did_ftplugin = 1

if !exists("*s:InsertTimestamp")
    function s:InsertTimestamp ()
        let timestamp = strftime('-- %Y-%m-%d %a %H:%M -- ')
        if len(getline('.')) >= 1
            call append('.', timestamp)
            call cursor(line('.')+1, 0)
            call cursor('.', len(getline('.')))
        else
            call setline('.', timestamp)
            call cursor('.', len(getline('.')))
        endif
    endfunction
endif

let s:insert_ticket_path = fnamemodify(resolve(expand('<sfile>:p')), ':h') . "/../InsertTickets.py"
:execute "py3file " . expand(s:insert_ticket_path)
if !exists("*s:InsertTickets")
    function s:InsertTickets()
        py3 insertTicketNames()
    endfunction
endif

" Enter timestamps to the file
nmap <buffer> <S-F3> :call <SID>InsertTickets()<CR>
imap <buffer> <S-F3> <Esc>:call <SID>InsertTickets()<CR>
nmap <buffer> <F3> :call <SID>InsertTimestamp()<CR>a
imap <buffer> <F3> <Esc>:call <SID>InsertTimestamp()<CR>a
nmap <buffer> <F4> :call <SID>InsertTimestamp()<CR>aPause<CR><Esc>
imap <buffer> <F4> <Esc>:call <SID>InsertTimestamp()<CR>aPause<CR>

" Execute the script that displays the work summary
" You will need to have the "log_work" script in your "PATH" environment variable
nmap <buffer> <F5> :w<CR>:! log_work -v -l %<CR>
imap <buffer> <F5> <Esc>:w<CR>:! log_work -v -l %<CR>
nmap <buffer> <S-F5> :w<CR>:! log_work -v --log_to_jira %<CR>
imap <buffer> <S-F5> <Esc>:w<CR>:! log_work -v --log_to_jira %<CR>
