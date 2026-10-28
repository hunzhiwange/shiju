# Synthetic retrieval diagnostics

Four independent standard-library JSON programs cover document/revision ingestion, cosine normalization, stable ordinal ranking, and exact document content. Each checker executes the submitted `/app/retrieve.py` with hidden synthetic inputs. Every child has gold, nop and three semantic negative controls; all fixtures are synthetic and require no embedding download.

`validation.json` binds the 20 fresh fixture controls to the checker, inputs, implementations and fixed diagnostic image. `preparation.json` records the separate official parent oracle/nop and eight fresh standard child controls under the calibrated suite execution identity. Both parent controls executed all two official tests; oracle passed and nop failed. Frozen runtime ELF probes passed in the fixed parent and diagnostic images.

The official parent uses `alexgshaw/mteb-retrieve:20260430`, manifest `sha256:8b3220cd25483014236076477603c7e02babfd670bf79150b5a1f0110ba662f9`. All compressed layer hashes and uncompressed diff IDs were verified; exact uncompressed size was 2,138,008,576 bytes. Upstream inputs, reference solution and scoring remain in the ignored source tree.

Raw calibration, dependency, resource, capacity and import evidence is retained under `evals/promptfoo/results/qwen38-suite/diagnostics/mteb-retrieve/`. Historical failures remain separate from the current standard controls. Future execution-identity changes require the suite preparation gate to revalidate evidence. No paid model calls were made.
