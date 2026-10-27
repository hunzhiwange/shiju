call setreg('a', ":s/ *, */,/g\<CR>:s/^ *//\<CR>:s/ *$//\<CR>")
call setreg('b', "0")
call setreg('c', "0gU$:s/,/;/g\<CR>A;OK\<Esc>")
:%normal! @a
:%normal! @b
:%normal! @c
:wq
