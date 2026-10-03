# linecop skill evals

Four evals in `evals.json`, run through skill-creator's loop: an executor
subagent per eval with and without the skill, each assertion graded, the two
arms compared. Run executor and grader on Sonnet, the same model for both arms.

## Fixture

`fixture/` is kvconf, a tiny Rust project built to hold the traps:

- `src/parser.rs` is 93% of its limit, so growing it needs a split up front.
- `count_mode: code` makes `wc -l` lie: parser.rs has more raw lines than its
  limit and still passes.
- `src/tables.rs` is generated and over its limit, so the honest fix is an
  override with a comment, not a split.

`tests/skill_fixture.rs` pins that state, so `just check` fails the moment the
fixture drifts. An ungated fixture rots silently: mindtape's skill sandbox
outlived two config migrations unnoticed in a gitignored scratch tree.

## What discriminates

Eval 1 separates the arms: without the skill, an agent writes into parser.rs,
then hits the limit and moves the code. Evals 0, 2 and 3 pass in both arms
once the prompt names linecop, because `--help` and the generated file's header
say enough. They guard against regressions; they are not evidence that the
skill helps.

## Running one

Copy the fixture to a scratch dir and hand that to the executor. Never run in
place: evals edit files and the fixture is tracked.

```sh
work=$(mktemp -d)
cp -a skills/linecop/evals/fixture "$work/project"
```
