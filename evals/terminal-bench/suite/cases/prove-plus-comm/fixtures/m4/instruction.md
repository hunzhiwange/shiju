Deliver a proof of commutativity that the trusted Coq compiler and kernel checker can verify.

Independent synthetic exercise m4. Coq's standard natural numbers and addition are used.
Fixed target: `forall n m : nat, n + m = m + n`.
Write only a tactic body to `/app/proof.txt`. The verifier constructs the theorem
itself from its private target, adds `Proof.` and `Qed.`, recompiles in a clean
directory and runs coqchk. Agent-supplied compiled artifacts are ignored.

Available imports: Coq.Arith.PeanoNat. Allowed tactic words: intros, induction,
as, simpl, rewrite, reflexivity, exact, apply, assumption. Allowed names:
n, m, k, IH, S, O, Nat.add_0_r, Nat.add_succ_r. Allowed punctuation:
period, semicolon, minus, plus, brackets, pipe, parentheses, colon, equals,
angle brackets, digits and whitespace. Use n, m as quantified names and k, IH
for the inductive predecessor and hypothesis. No comments, imports, declarations,
axioms, assumptions, admit, Admitted, sorry, external commands or added goals.
`goal.txt` is public context; modifying it cannot change the verifier's target.
No text report or fabricated .vo is accepted. A rejected syntactic candidate is
not evidence that Coq ran. Missing coqc/coqchk is an infrastructure error.
You may test by placing the body between Proof./Qed. in a local .v file and
running coqc then coqchk; the verifier independently repeats both.
