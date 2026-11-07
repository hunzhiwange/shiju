# iced_term local patch

The baseline is MIT-licensed. Cargo metadata makes the library standalone
so the desktop can use it as a path dependency.
[doyouhaobaby/iced_term at 4cf92c2](https://github.com/doyouhaobaby/iced_term/tree/4cf92c247879d1cce1b8bdba5dd301d844179ac5)

The terminal widget requests native IME while focused and anchors the candidate
area at its caret. Iced renders preedit text separately from the PTY. A commit
writes the original UTF-8 bytes once through the existing backend. Focus loss,
window deactivation, and switching terminals clear the pending preedit.
