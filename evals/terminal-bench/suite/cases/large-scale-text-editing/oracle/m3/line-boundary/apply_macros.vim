call setreg('a', ":s/ *, */,/g\<CR>:s/^ *//\<CR>:s/ *$//\<CR>")
call setreg('b', ":s/^\\([^,]*\\),\\([^,]*\\),\\([^,]*\\)$/\\3,\\2,\\1/\<CR>")
call setreg('c', "j0gU$:s/,/;/g\<CR>A;OK\<Esc>")
:%normal! @a
:%normal! @b
:%normal! @c
:wq
