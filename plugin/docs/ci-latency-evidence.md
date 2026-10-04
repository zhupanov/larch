# End-to-end CI latency and runner-cost evidence

This is the evidence record for [#8330](https://github.com/character-ai/larch/issues/8330), the final leaf of [#8324](https://github.com/character-ai/larch/issues/8324), and the later [#8484](https://github.com/character-ai/larch/issues/8484) merge-group cohort. It preserves raw GitHub Actions timestamps, cache observations, and cohort rules rather than rounded workflow-UI durations.

The retained #8330 snapshot time is 2026-08-10. Its observations were collected after the last prerequisite merged at `282d0ca8bad0379ee44b8f95bb2e5b7ec45515d8`.

## Post-#8482 warm merge-group cohort

Measurement date: 2026-08-14. [PR #8482](https://github.com/character-ai/larch/pull/8482) merged through cold run [31760348131](https://github.com/character-ai/larch/actions/runs/31760348131); its trusted-main publisher, [31760751229](https://github.com/character-ai/larch/actions/runs/31760751229), then completed successfully. The next target-cache miss is retained below as an exclusion. The next three successful direct-hit `merge_group` runs are the eligible cohort: no manual dispatch is included.

All three eligible runs used `CI`, attempt 1, and 41 successful actual runner jobs. An actual runner job has a non-empty `runner_name`; skipped declarations do not enter duration sums. Queue delay is the earliest actual start minus `created_at`; trigger-to-last-required is the latest required completion minus `created_at`; active DAG span is the difference between those two actual timestamps; and summed runner time is the sum of all actual runner durations. `python-tests-gate` was the latest required completion in every sample.

| Run | SHA | Event | Created (UTC) | Attempt | Queue | Cache evidence | Result |
| --- | --- | --- | --- | ---: | ---: | --- | --- |
| [31773677023](https://github.com/character-ai/larch/actions/runs/31773677023) | `39275b6` | `merge_group` | 2026-08-14T05:39:08Z | 1 | 4 s | direct coverage-target hit, 1,398,468,608 B in 9 s; Cargo inputs, nextest, and LLVM-Cov hits | success |
| [31776106272](https://github.com/character-ai/larch/actions/runs/31776106272) | `2663e36` | `merge_group` | 2026-08-14T06:23:20Z | 1 | 4 s | direct coverage-target hit, 1,398,472,704 B in 8 s; Cargo inputs, nextest, and LLVM-Cov hits | success |
| [31778104389](https://github.com/character-ai/larch/actions/runs/31778104389) | `4f0a39c` | `merge_group` | 2026-08-14T06:57:12Z | 1 | 4 s | direct coverage-target hit, 1,398,472,704 B in 7 s; Cargo inputs, nextest, and LLVM-Cov hits | success |

Every raw value used by the job-level medians is in seconds.

“Python matrix sum” is the sum of the then-current 20 `python-tests (3.11, N)` runner durations. “Harness” is the then-current five `test-harnesses (N)` jobs.

| Run | Trigger→last required | Active DAG | Sum runner | Slowest harness | Sum harness | Python matrix sum | `rust-full` | Rust gate path | `rust-lint` | `gitleaks` | `agent-sync` | Final required job |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| [31773677023](https://github.com/character-ai/larch/actions/runs/31773677023) | 368 | 364 | 2,531 | 212 | 351 | 941 | 343 | 355 | 80 | 258 | 211 | `python-tests-gate` |
| [31776106272](https://github.com/character-ai/larch/actions/runs/31776106272) | 326 | 322 | 2,403 | 213 | 363 | 909 | 289 | 304 | 77 | 224 | 216 | `python-tests-gate` |
| [31778104389](https://github.com/character-ai/larch/actions/runs/31778104389) | 378 | 374 | 2,463 | 234 | 376 | 932 | 345 | 360 | 81 | 177 | 216 | `python-tests-gate` |
| Median | **368** | **364** | **2,463** | **213** | **363** | **932** | **343** | **355** | **80** | **224** | **216** | `python-tests-gate` |

The `rust-full` timing artifacts provide the raw phase values below. “Candidate staging” is the target-cache save/stage phase; it is skipped when every primary key is already present, rather than being treated as a miss.

| Run | Timing TSV | Restore | Compilation | Nextest | Repository policy | Coverage report | Candidate staging | Coverage end-to-end | Job total |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 31773677023 | [9209156949](https://github.com/character-ai/larch/actions/runs/31773677023/artifacts/9209156949) | 16 | 76 | 80 | 97 | 30 | 0 s skipped: all primary keys hit | 286 | 334 |
| 31776106272 | [9210006658](https://github.com/character-ai/larch/actions/runs/31776106272/artifacts/9210006658) | 15 | 62 | 68 | 72 | 25 | 0 s skipped: all primary keys hit | 230 | 276 |
| 31778104389 | [9210755137](https://github.com/character-ai/larch/actions/runs/31778104389/artifacts/9210755137) | 15 | 74 | 79 | 98 | 30 | 1 s skipped: all primary keys hit | 284 | 330 |
| Median | — | **15** | **74** | **79** | **97** | **30** | **0 s** | **284** | **330** |

The post-#8482 exclusions remain visible and are not pooled with the median:

| Run | Event | Queue | Trigger→last required | Cache observation | Why excluded |
| --- | --- | ---: | ---: | --- | --- |
| [31760348131](https://github.com/character-ai/larch/actions/runs/31760348131) | `merge_group` | 4 s | 452 s | coverage-target miss, 0 B restored; 150 s compilation; candidate staged for trusted-main publication | #8482 changed the Cargo graph |
| [31771030876](https://github.com/character-ai/larch/actions/runs/31771030876) | `merge_group` | 4 s | 427 s | coverage-target miss, 0 B restored; 149 s compilation; candidate staged for trusted-main publication | cache miss before the eligible direct-hit sequence |

No rerun, cancellation, or runner-queue outlier occurred in this collection window: every listed run used attempt 1 and had a 4 s queue. Queue delay is therefore reported separately from the 364 s median active DAG span.

The observed required-job envelope is `rust-full` → `rust-coverage` → `rust-gate` → `python-tests-gate`: the Rust gate path has a 355 s median and `python-tests-gate` completed last at a 368 s median. This names the observed critical path without claiming a separate dependency-graph proof. The 368 s trigger-to-last-required median is below the seven-minute target, so this cohort does not admit the dependent optimization issues [#8485](https://github.com/character-ai/larch/issues/8485), [#8486](https://github.com/character-ai/larch/issues/8486), [#8487](https://github.com/character-ai/larch/issues/8487), and [#8488](https://github.com/character-ai/larch/issues/8488) under the #8475 decision rule; each needs new evidence and approval before implementation.

## #8486 matrix-sizing decision

The operator explicitly authorized serial implementation of the remaining open leaves after the #8484 assessment. That authorization permits this narrowly measured change despite the prior admission result; it does not turn the #8484 cohort into a general performance claim.

The three merge-group runs above are the planning cohort. Their Python `call` rows total 348.12 s, 346.28 s, and 346.21 s. The canonical LPT planner considered 4 and 8 output shards from the same 1,525 observed nodeids: four produces 379/382/382/382 nodeids and 87.440/87.440/87.435/87.440 modeled call seconds; eight produces 188–192 nodeids and 43.715–43.720 seconds. Four is selected because it removes 16 Python runners while its modeled test work remains far below the 355 s median Rust-gate envelope. A rebase onto newer main removed three retired nodeids from the generated map, preserving the upstream deletion; the checked-in four-shard map therefore has 1,522 entries. The planner now updates the assignment map and all three CI count fields atomically, so a requested output width cannot leave the map and matrix inconsistent.

The specifically assessed marker-heavy Python example was `tests/issue/test_plan_marker_ownership.py::test_runtime_plan_markers_use_the_shared_grammar_owner`, with calls of 10.65 s, 6.12 s, and 9.96 s (median 9.96 s). At 11.4% of the four-way modeled maximum it is not a material tail. Marker prefiltering would add collection and failure-diagnosis complexity without a justified critical-path benefit, so this change keeps the existing single pytest collection path.

The harness measurements show why the five-cell matrix was reduced to two. Cells 1, 2, and 5 were empty in every cohort member, yet consumed startup time. The Rust-hook leg keeps its isolated cache/bootstrap diagnostic surface; every remaining direct Bash leaf stays together in the second leg.

| Run | Empty cells (s) | Rust-hook leg (s) | Other-leaf leg (s) |
| --- | --- | ---: | ---: |
| 31773677023 | 1=13, 2=10, 5=13 | 212 | 103 |
| 31776106272 | 1=15, 2=14, 5=9 | 213 | 112 |
| 31778104389 | 1=12, 2=10, 5=11 | 234 | 109 |

This preserves exactly-once harness ownership and keeps the slower existing harness leg unchanged; it removes 36 s, 38 s, and 33 s of measured empty-runner time. The stable `test-harnesses-gate` and `python-tests-gate` context names remain unchanged.

### Post-change controlled cohort

Three serial, successful full-path `workflow_dispatch` runs exercised the final four-Python/two-harness candidate at [`fa178c45e`](https://github.com/character-ai/larch/commit/fa178c45ebd17889f10370e6cca70dfe5cec607b). Every actual runner job succeeded, all runs used attempt 1, and each had exactly 22 actual runner jobs. Values below are seconds; the final job remained the Rust-backed `python-tests-gate` consumer, not a Python matrix leg.

| Run | Trigger→last | Active DAG | Sum runner | Python sum / max | Harness sum / max | Rust full | Rust gate path | Final job |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| [31793144304](https://github.com/character-ai/larch/actions/runs/31793144304) | 383 | 378 | 1,778 | 461 / 122 | 229 / 141 | 335 | 350 | `python-tests-gate` |
| [31794577585](https://github.com/character-ai/larch/actions/runs/31794577585) | 401 | 396 | 1,840 | 477 / 128 | 264 / 167 | 350 | 365 | `python-tests-gate` |
| [31795357826](https://github.com/character-ai/larch/actions/runs/31795357826) | 390 | 386 | 1,838 | 476 / 125 | 277 / 176 | 345 | 359 | `python-tests-gate` |
| Median | 390 | 386 | 1,838 | 476 / 125 | 264 / 167 | 345 | 359 | `python-tests-gate` |

Against the planning-cohort medians, the candidate cuts summed Python runner time from 932 s to 476 s and harness runner time from 363 s to 264 s, while reducing actual runner jobs from 41 to 22. Python matrix maximum rises from 53 s to 125 s as intended when each surviving cell owns five times as many tests, but it finishes before the Rust producer and therefore does not make the matrix a required-path tail.

Collection remains the existing single pytest collection path: every new Python cell reported `collected 6,601 items`, for 26,404 item-collections across the matrix, versus 20 × 6,608 = 132,160 in the prior matrix (an 80% reduction in repeated collection work). The per-run maximum pytest elapsed time was 109.61 s, 110.95 s, and 112.93 s (median 110.95 s). This is why marker-byte prefiltering remains out of scope: the assessed marker tail is not material, and the four surviving full-collection cells remain safely before Rust.

To distinguish the matrix change from live Rust-producer variance, the first two candidates were paired sequentially with the exact parent revision [`cb9c2612c`](https://github.com/character-ai/larch/commit/cb9c2612c2d157ea51256f7ad073b900823df27d) using its 20-Python/five-harness matrix. The controls were [31792414259](https://github.com/character-ai/larch/actions/runs/31792414259) (379 s trigger-to-last, 2,288 s summed runner, 915 s Python, 299 s harness, 333 s `rust-full`) and [31793876810](https://github.com/character-ai/larch/actions/runs/31793876810) (389 s, 2,277 s, 893 s, 292 s, and 341 s respectively). Their paired candidates retain the large runner-time reduction; the 383–401 s whole-workflow values track the independent `rust-full` and Rust-backed integration tail rather than a Python matrix dependency. The raw cohort does not claim a lower end-to-end median than the earlier 368 s merge-group cohort; it records the remaining critical path candidly. All samples remain below the seven-minute umbrella threshold, and the retained optimization is justified by its directly measured runner-time win without weakening any required check or failure-diagnosis surface.

## Result and scope

Three sequential, successful full-path `workflow_dispatch` runs on `refs/heads/main` at that SHA have a median trigger-to-last-job time of 337 s and a median summed runner time of 1,997 s. Every numerical threshold in the leaf is below its limit in that controlled cohort.

That is an observation, not a production-push target declaration. The [production-main-run rule](rust-testing.md#production-main-run-evidence) requires three comparable warm successful `push` runs after the repair; this snapshot has one: [run 31418264078](https://github.com/character-ai/larch/actions/runs/31418264078). Manual dispatches do not substitute for the remaining two pushes. The umbrella therefore remains open for the production-push cohort rather than claiming a pass from an ineligible sample.

## Measurement method

Raw data comes from the GitHub Actions run record and `GET /repos/character-ai/larch/actions/runs/{run}/jobs?per_page=100`. The `per_page=100` parameter is material: it prevents the default page size from omitting jobs. Timestamps are GitHub's whole-second `created_at`, `started_at`, and `completed_at` fields.

An *actual runner job* has a non-empty `runner_name` and successful conclusion. Declared jobs which GitHub marks skipped have no runner time and are excluded from sums and spans.

| Measure | Raw calculation |
| --- | --- |
| Queue delay | earliest actual `started_at` minus run `created_at` |
| Trigger to last job | latest actual `completed_at` minus run `created_at` |
| Active DAG span | latest actual `completed_at` minus earliest actual `started_at` |
| Summed runner time | sum of `completed_at - started_at` for actual runner jobs |
| Slowest / summed harness | maximum / sum of actual `test-harnesses (N)` durations |
| Rust gate path | `rust-gate.completed_at - rust-full.started_at` |

“Final critical-path job” below is the job with the last observed completion timestamp. It is an execution-envelope observation, not a claim that a separate dependency-graph analyzer proved the only logical path.

## Controlled main-ref cohort

All three samples used the `CI` workflow at `refs/heads/main`, the same SHA, and attempt 1. Each was dispatched only after the prior sample completed, so no stacked-push or concurrent-run effect is included. No cohort member was cancelled, rerun, cold, or a cache miss: every run has attempt 1 and a direct warm target-cache hit. Every actual runner job (41 in each run) concluded `success`; the runner labels were `ubuntu-24.04` and `ubuntu-latest` in every sample.

| Run | SHA | Event | Created (UTC) | Attempt | Queue | Cache class | Result |
| --- | --- | --- | --- | ---: | ---: | --- | --- |
| [31418859465](https://github.com/character-ai/larch/actions/runs/31418859465) | `282d0ca` | `workflow_dispatch` | 2026-08-10T18:23:54Z | 1 | 7 s | warm direct coverage-target hit; Cargo input/nextest/LLVM-Cov hits; gitleaks binary hit; dispatch read-only | success |
| [31419539198](https://github.com/character-ai/larch/actions/runs/31419539198) | `282d0ca` | `workflow_dispatch` | 2026-08-10T18:31:57Z | 1 | 6 s | warm direct coverage-target hit; Cargo input/nextest/LLVM-Cov hits; gitleaks binary hit; dispatch read-only | success |
| [31420286318](https://github.com/character-ai/larch/actions/runs/31420286318) | `282d0ca` | `workflow_dispatch` | 2026-08-10T18:40:45Z | 1 | 5 s | warm direct coverage-target hit; Cargo input/nextest/LLVM-Cov hits; gitleaks binary hit; dispatch read-only | success |

The raw end-to-end values are seconds. In this historical cohort, “Harness” means the then-current five `test-harnesses (N)` jobs; “Rust producer” is `rust-full` on main.

| Run | Trigger→last | Active DAG | Sum runner | Slowest harness | Sum harness | Rust producer | Rust gate path | `lint` | `rust-lint` | `gitleaks` | Final critical-path job |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| [31418859465](https://github.com/character-ai/larch/actions/runs/31418859465) | 337 | 330 | 1,997 | 51 | 180 | 305 | 322 | 48 | 59 | 27 | `python-tests-gate` |
| [31419539198](https://github.com/character-ai/larch/actions/runs/31419539198) | 342 | 336 | 2,195 | 48 | 152 | 305 | 320 | 47 | 64 | 24 | `python-tests-gate` |
| [31420286318](https://github.com/character-ai/larch/actions/runs/31420286318) | 286 | 281 | 1,946 | 52 | 173 | 243 | 263 | 51 | 72 | 25 | `python-tests-gate` |
| Median | **337** | **330** | **1,997** | **51** | **173** | **305** | **320** | **48** | **64** | **25** | `python-tests-gate` |

No rounded UI value is used for these medians: with three samples, each is the middle raw value.

### Coverage and cache timing artifacts

Each timing TSV reports direct coverage-target and all three Cargo-related cache hits. The dispatch policy correctly prevents cache publication; that is not a cache miss.

| Run | Timing TSV | Restore total | Target restore | Compilation | Test execution | Repository policy | Report | `end-to-end-total-16` | Job total | Save outcome |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 31418859465 | [9074665679](https://github.com/character-ai/larch/actions/runs/31418859465/artifacts/9074665679) | 8 s | hit, 5 s, 1,348,857,856 B | 93 s | 63 s | 80 s | 25 s | 263 s | 299 s | 0 s skipped: `workflow_dispatch-read-only` |
| 31419539198 | [9074925405](https://github.com/character-ai/larch/actions/runs/31419539198/artifacts/9074925405) | 10 s | hit, 5 s, 1,348,857,856 B | 90 s | 58 s | 83 s | 25 s | 259 s | 299 s | 0 s skipped: `workflow_dispatch-read-only` |
| 31420286318 | [9075168617](https://github.com/character-ai/larch/actions/runs/31420286318/artifacts/9075168617) | 11 s | hit, 6 s, 1,348,857,856 B | 72 s | 57 s | 48 s | 18 s | 196 s | 236 s | 0 s skipped: `workflow_dispatch-read-only` |

The gitleaks job used the verified binary cache in every sample. In run 31419539198, the raw log records a hit for the 2,871,356-byte `gitleaks` cache and 15 s checkout, 0 s Rust bootstrap, 0 s verified-tool preparation, 4 s working-tree scan, and 0 s history scan. Its 24 s job duration is the raw value above.

### Threshold comparison

| Leaf target | Controlled-cohort median | Observation |
| --- | ---: | --- |
| Trigger to last job ≤ 392 s | 337 s | below limit |
| Summed runner ≤ 2,836 s | 1,997 s | below limit |
| Slowest harness ≤ 300 s | 51 s | below limit |
| Rust gate path ≤ 456 s | 320 s | below limit |
| `rust-lint` < 300 s | 64 s | below limit |
| Retain all checks | 41/41 actual runner jobs successful in each run | retained |

The table is deliberately an observation: it cannot satisfy the three-`push` production requirement.

### Supporting post-prerequisite production push

[Run 31418264078](https://github.com/character-ai/larch/actions/runs/31418264078) is the single successful post-prerequisite `push` sample. It ran at `282d0ca8bad0379ee44b8f95bb2e5b7ec45515d8`, was created at 2026-08-10T18:16:45Z, had attempt 1, used both runner labels, and had 41/41 successful actual runner jobs. Its queue was 4 s; trigger-to-last 354 s; active span 350 s; summed runner 1,965 s; slowest/summed harness 55/176 s; `rust-full` 317 s; Rust gate path 332 s; `lint` 51 s; `rust-lint` 70 s; `gitleaks` 26 s; and its last completion was `python-tests-gate`.

Its [timing TSV](https://github.com/character-ai/larch/actions/runs/31418264078/artifacts/9074434915) records a direct target-cache hit in 6 s restoring 1,348,861,952 B, with all Cargo cache inputs hit and cache save skipped because all primary keys hit. It corroborates the controlled cohort but is not pooled with it because the event differs. Cold/miss, cancelled, retried, or stacked-push runs are likewise not eligible for the required warm production median.

## Historical reconciliation

### Original #8211 twelve-run cohort

The exact twelve successful `push` runs identified by [#8211](https://github.com/character-ai/larch/issues/8211) produce these raw job durations when recalculated from Actions job timestamps:

| Run | SHA | Event | Created (UTC) | `rust-lint` | `rust-coverage` |
| --- | --- | --- | --- | ---: | ---: |
| [31136683447](https://github.com/character-ai/larch/actions/runs/31136683447) | `4e74540` | `push` | 2026-08-07T01:02:01Z | 290 | 532 |
| [31135807320](https://github.com/character-ai/larch/actions/runs/31135807320) | `98b98c7` | `push` | 2026-08-07T00:46:34Z | 499 | 636 |
| [31135160651](https://github.com/character-ai/larch/actions/runs/31135160651) | `97b2036` | `push` | 2026-08-07T00:35:10Z | 489 | 529 |
| [31132292064](https://github.com/character-ai/larch/actions/runs/31132292064) | `e7a41fc` | `push` | 2026-08-06T23:46:47Z | 408 | 615 |
| [31132005623](https://github.com/character-ai/larch/actions/runs/31132005623) | `51277e5` | `push` | 2026-08-06T23:41:14Z | 479 | 650 |
| [31091531282](https://github.com/character-ai/larch/actions/runs/31091531282) | `18d98ff` | `push` | 2026-08-06T10:01:15Z | 334 | 540 |
| [31088672342](https://github.com/character-ai/larch/actions/runs/31088672342) | `d6c392c` | `push` | 2026-08-06T09:20:24Z | 483 | 608 |
| [31086731602](https://github.com/character-ai/larch/actions/runs/31086731602) | `a6127b4` | `push` | 2026-08-06T08:53:04Z | 307 | 622 |
| [31084686100](https://github.com/character-ai/larch/actions/runs/31084686100) | `4d71a53` | `push` | 2026-08-06T08:23:19Z | 474 | 521 |
| [31083821408](https://github.com/character-ai/larch/actions/runs/31083821408) | `c7c543a` | `push` | 2026-08-06T08:10:33Z | 287 | 605 |
| [31083278265](https://github.com/character-ai/larch/actions/runs/31083278265) | `712d4b0` | `push` | 2026-08-06T08:02:40Z | 467 | 524 |
| [31081377729](https://github.com/character-ai/larch/actions/runs/31081377729) | `4070029` | `push` | 2026-08-06T07:33:26Z | 502 | 608 |
| Exact median | — | — | — | **470.5 s** | **606.5 s** |

The issue text's historical `460/608` pair is not treated as a raw-timestamp fact. `608` is consistent with whole-second rounding of the recalculated 606.5 s coverage median; `460` does not result from this exact twelve-run cohort. The original calculation inputs for `460` were not retained, so this record does not infer whether it used a different selection or method.

### Seven-run reference before #8251 merged

The seven successful main `push` runs immediately before PR #8251 merged at 2026-08-08T09:47:29Z reproduce the umbrella's common end-to-end reference. These measures exist across the older and newer producer topologies; run 31247270073 used the older `rust-coverage` producer name.

| Run | SHA | Queue | Trigger→last | Active DAG | Sum runner | Slowest harness | Sum harness | Final job |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| [31250456341](https://github.com/character-ai/larch/actions/runs/31250456341) | `e306fd4` | 4 | 393 | 389 | 2,473 | 239 | 546 | `python-tests-gate` |
| [31249860854](https://github.com/character-ai/larch/actions/runs/31249860854) | `6b5ec65` | 4 | 402 | 398 | 2,557 | 231 | 569 | `python-tests-gate` |
| [31249122221](https://github.com/character-ai/larch/actions/runs/31249122221) | `f0cfa4f` | 4 | 387 | 383 | 2,483 | 232 | 555 | `python-tests-gate` |
| [31248746198](https://github.com/character-ai/larch/actions/runs/31248746198) | `0cddbc9` | 3 | 386 | 383 | 2,536 | 232 | 556 | `python-tests-gate` |
| [31248343047](https://github.com/character-ai/larch/actions/runs/31248343047) | `655e379` | 4 | 397 | 393 | 2,506 | 225 | 539 | `python-tests-gate` |
| [31247754562](https://github.com/character-ai/larch/actions/runs/31247754562) | `1dc4844` | 4 | 312 | 308 | 2,436 | 247 | 567 | `python-tests-gate` |
| [31247270073](https://github.com/character-ai/larch/actions/runs/31247270073) | `dc4c9a4` | 3 | 392 | 389 | 2,471 | 221 | 531 | `python-tests-gate` |
| Median | — | **4** | **392** | **389** | **2,483** | **232** | **555** | `python-tests-gate` |

### Recent warm full-push regression window

The following six successful `push` runs were the recent warm full-path regression window carried by #8324. All have direct coverage-target cache hits. Their target restore time/bytes in row order are 5 s/1,348,853,760 B; 7 s/1,348,857,856 B; 9 s/1,348,861,952 B; 4 s/1,348,857,856 B; 8 s/1,348,857,856 B; and 5 s/1,348,853,760 B. They are regression evidence, not a replacement for the post-repair production cohort.

| Run | SHA | Queue | Trigger→last | Active DAG | Sum runner | Slowest harness | Sum harness | Rust producer | Rust gate path | `rust-lint` | `gitleaks` | Final job |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| [31317079211](https://github.com/character-ai/larch/actions/runs/31317079211) | `e091925` | 4 | 435 | 431 | 3,818 | 423 | 2,058 | 298 | 310 | 64 | 211 | `test-harnesses-gate` |
| [31314856995](https://github.com/character-ai/larch/actions/runs/31314856995) | `2b0318b` | 5 | 452 | 447 | 3,782 | 433 | 1,983 | 293 | 307 | 59 | 215 | `test-harnesses-gate` |
| [31312377141](https://github.com/character-ai/larch/actions/runs/31312377141) | `bf912d0` | 4 | 430 | 426 | 3,837 | 417 | 2,042 | 301 | 323 | 59 | 203 | `test-harnesses-gate` |
| [31309885781](https://github.com/character-ai/larch/actions/runs/31309885781) | `1636564` | 5 | 439 | 434 | 3,730 | 428 | 1,971 | 280 | 300 | 63 | 205 | `test-harnesses-gate` |
| [31307943093](https://github.com/character-ai/larch/actions/runs/31307943093) | `894d86e` | 4 | 451 | 447 | 3,705 | 440 | 1,989 | 247 | 263 | 53 | 205 | `test-harnesses-gate` |
| [31306299339](https://github.com/character-ai/larch/actions/runs/31306299339) | `21e30f9` | 4 | 429 | 425 | 3,780 | 417 | 2,038 | 264 | 279 | 60 | 211 | `test-harnesses-gate` |
| Median | — | **4** | **437** | **432.5** | **3,781** | **425.5** | **2,013.5** | **286.5** | **303.5** | **59.5** | **208** | `test-harnesses-gate` |

## Pull-request selector evidence

Selector evidence includes a full control, an eligible partial classification, and an enforced safe skip. It is not mixed into main-ref or production-push medians.

| Class | Run | Selector observation | Raw execution result | Queue / trigger / active / runner | Final job |
| --- | --- | --- | --- | --- | --- |
| Full control | [31417763547](https://github.com/character-ai/larch/actions/runs/31417763547) | proposed `full`, effective `full`, `selector-proposed-full`; workflow/action inputs were global | `rust-full` 261 s; Rust gate path 292 s | 4 / 319 / 315 / 1,929 s | `python-tests-gate` |
| Partial eligible | [31293686433](https://github.com/character-ai/larch/actions/runs/31293686433) | proposed `partial` for `larch-cli` and `larch-lint`; effective `full`, `partial-observation-window-open` | actual `rust-full` 276 s; Rust gate path 289 s | 1,001 / 1,421 / 420 / 3,678 s | `test-harnesses-gate` |
| Safe skip | [31304987045](https://github.com/character-ai/larch/actions/runs/31304987045) | proposed/effective `skip`, `selector-proposed-skip`; trusted policy proof valid | `rust-skip` 68 s; Rust gate path 82 s | 3 / 421 / 418 / 3,368 s | `test-harnesses-gate` |

The partial sample is a valid selector-classification observation but not a comparable latency sample: its 1,001 s initial queue is kept separate from the 420 s active span. The partial enforcement flag remains `false`, so GitHub correctly ran a full producer.

The safe-skip sample shows why a narrower Rust selection alone does not prove a user-visible critical-path improvement. In run 31304987045, `rust-skip` ended at 2026-08-09T09:04:48Z, `python-tests-gate` ended at 09:05:18Z, and `test-harnesses-gate` did not end until 09:10:14Z. The final job remained the harness gate. This is a timestamp observation, not a counterfactual estimate of selector savings.

The underlying selection records are [full 9074079490](https://github.com/character-ai/larch/actions/runs/31417763547/artifacts/9074079490), [partial 9032423030](https://github.com/character-ai/larch/actions/runs/31293686433/artifacts/9032423030), and [skip 9035649436](https://github.com/character-ai/larch/actions/runs/31304987045/artifacts/9035649436).

## Native issue graph and follow-up

The native dependency query made before #8330 closes reported these direct relationships:

- #8324 has sub-issues #8325, #8326, #8327, #8328, #8329, and #8330; its `blockedBy` set is the same.
- #8330 has parent #8324 and blocks #8324.
- #8330 is blocked by #8320, #8329, #8328, and #8327, all closed at collection time.

The merge-time read-back is recorded on #8324 so it can show #8330's closed state rather than freezing this pre-close snapshot. Two additional warm, successful, full-path `push` runs on `refs/heads/main` are required before a production target pass can be claimed. Until then, #8324 remains open.
