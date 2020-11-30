" This detector detects files of type "ft=work" by their ".work" extension.
" You should copy it to "$VIMRUNTIME/ftdetect/work.vim".
" For Unix this is usually "~/.vim/ftdetect/work.vim"
" You can find out your "$VIMRUNTIME" by typing ":set runtimepath?" into vim.

au BufRead,BufNewFile *.work        set filetype=work
