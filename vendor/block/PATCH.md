# block 0.1.6 compatibility patch

Source: https://github.com/SSheldon/rust-block at
642ea4a4a5853a21b55b05c34832a5f1bb1af61c (MIT).

The published source and upstream test helper are retained. The Rust changes replace the empty `Class` enum with an inhabited opaque C-layout struct and make the existing C ABI explicit
on extern declarations.
Only pointers to this type are used; the external symbol and block ABI stay unchanged.
This fixes `uninhabited_static` (rust-lang/rust#74840) without suppressing the lint.
Remove this override when upstream publishes a compatible fix.
