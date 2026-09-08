# Rust CI selector historical classification baseline

This record distinguishes historical classifier replays from the live
observation window that controls non-full Rust CI enforcement. Historical rows
are useful to exercise the final deterministic classifier against immutable
pull-request merge candidates, but they are not live selected-path evidence and
cannot authorize a non-full lane.

## Historical classification baseline

For each row, the deterministic selector ran with the recorded PR base and
tested merge SHA in an isolated candidate checkout. The mode is therefore an
executable classifier result. The linked required full Rust backstop also
completed successfully for every candidate.

The replay command was:

```bash
CLAUDE_PLUGIN_ROOT=<trusted-base> LARCH_BINARY=<trusted-base>/target/debug/larch \
  <trusted-base>/scripts/larch.sh ci rust-select \
  --event-name pull_request \
  --base-sha <recorded-base> \
  --head-sha <tested-merge-candidate> \
  --repo-root <detached-candidate-worktree>
```

The selector source was the implementation in this change. In CI it executes
from the trusted pull-request base while it reads the candidate checkout as
data. Replaying immutable candidates lets this record compare the final
classifier with exact trees that had a successful full backstop.

| PR | Tested merge candidate | Replayed decision | Proposed scope | Full backstop | Result |
|---|---|---|---|---|---|
| [#8002](https://github.com/character-ai/larch/pull/8002) | `f0c40ddc21ae3e73fce3b0e8308bee41a57c0839` | `partial` | `larch-cli`; format, selected Clippy/tests/doctests, candidate policy and plugin validation | [`rust-coverage`, 230 s](https://github.com/character-ai/larch/actions/runs/30301479391/job/90095217754); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/30301479391/job/90096148397) | full backstop succeeded |
| [#7901](https://github.com/character-ai/larch/pull/7901) | `bdb9900d204c82ed301647d1dfaae607962638a9` | `partial` | `larch-cli`; format, selected Clippy/tests/doctests, candidate policy and plugin validation | [`rust-coverage`, 140 s](https://github.com/character-ai/larch/actions/runs/29796763651/job/88529560397); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/29796763651/job/88529880499) | full backstop succeeded |
| [#7845](https://github.com/character-ai/larch/pull/7845) | `545d81c5fc5b082025413fe7360a16bff182e7a0` | `partial` | `larch-cli`, `larch-lint`; format, selected Clippy/tests/doctests, candidate policy and plugin validation | [`rust-coverage`, 126 s](https://github.com/character-ai/larch/actions/runs/29721888005/job/88286437364); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/29721888005/job/88286756784) | full backstop succeeded |
| [#8039](https://github.com/character-ai/larch/pull/8039) | `60440c1e50e0f359e93e6a75f4425fd18b6edd8d` | `skip` | audited `plugin/`, `python/`, and `skills/` owners; trusted-main policy and plugin validation | [`rust-coverage`, 205 s](https://github.com/character-ai/larch/actions/runs/30849578510/job/91806054206); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/30849578510/job/91806928458) | full backstop succeeded |
| [#8053](https://github.com/character-ai/larch/pull/8053) | `d02eca8726e9f9b6eb8ae86a9903ded1a4ea85dd` | `skip` | audited `agent-lint.toml` owner; trusted-main repository policy | [`rust-coverage`, 206 s](https://github.com/character-ai/larch/actions/runs/30980712688/job/92224321286); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/30980712688/job/92224921450) | full backstop succeeded |
| [#8024](https://github.com/character-ai/larch/pull/8024) | `a216be3438a2ba9dbc4bd3853ca4a6f3f0c9d2e6` | `skip` | audited `plugin/`, `python/`, and `skills/` owners; trusted-main policy and plugin validation | [`rust-coverage`, 204 s](https://github.com/character-ai/larch/actions/runs/30741434838/job/91479479364); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/30741434838/job/91479781196) | full backstop succeeded |

The baseline contains six independent pull requests: three `partial` and three
`skip`. Each replay selects a useful non-full class, and every corresponding
full backstop passed. It records zero historical full-backstop failures for
those classifications.

This is not a claim that the final `rust-partial` or `rust-skip` workflow jobs
executed in those historical runs; they predate this topology. It therefore
does not establish a live selected-path false-safe rate or a post-enforcement
duration. In particular, a local replay of #8002 against its historical CLI
cannot run the final policy command because that historical binary predates the
command surface. The exact command and ownership contracts are covered by
deterministic selector and workflow tests.

## Live observation contract

Both live windows are complete. `RUST_CI_PARTIAL_ENFORCEMENT` and
`RUST_CI_SKIP_ENFORCEMENT` are `true` after the completed class-specific
windows below. A proposed non-full mode executes only after the trusted-main
policy artifact validates; a cache or verification failure remains `full`.
The topology-changing pull request that enables a class is itself a global
`full` input and cannot be counted as that class's selected-path result.

Before a reviewed workflow change enables an unproven class, append at least
three independent live pull-request rows to this document. Each row must
include:

- a distinct pull-request number and tested merge candidate;
- the uploaded `rust-ci-selection` artifact's proposed mode and
  observation-window effective full reason;
- successful full-mode producer, `rust-coverage`, and `rust-gate` job links;
- an explicit false-safe or false-full comparison result; and
- the full-backstop duration plus, when the class is later enabled, the
  selected-path duration with its runner and cache conditions.

### Completed skip observation window

These are three distinct, ordinary docs-only pull requests. The linked
selection job uploaded the listed effective-decision record; every full
backstop in the same run passed.

| PR | Tested merge candidate | Proposed/effective decision | Full backstop | Comparison |
|---|---|---|---|---|
| [#8247](https://github.com/character-ai/larch/pull/8247) | `4d8d98e0a583f00e14aa8064124390289f873cab` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31248043914/job/93079779676): `skip` → `full`; `skip-observation-window-open`; `observation_only=true` | [`rust-full`, 345 s](https://github.com/character-ai/larch/actions/runs/31248043914/job/93079841037); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31248043914/job/93080391569); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31248043914/job/93080402605) | false-safe: none observed; false-full: not assessed while `full` was effective |
| [#8248](https://github.com/character-ai/larch/pull/8248) | `b9f8cad8070535181dc8e369b1f792c8090a1f86` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31248368102/job/93080605950): `skip` → `full`; `skip-observation-window-open`; `observation_only=true` | [`rust-full`, 352 s](https://github.com/character-ai/larch/actions/runs/31248368102/job/93080650942); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31248368102/job/93081184191); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31248368102/job/93081194121) | false-safe: none observed; false-full: not assessed while `full` was effective |
| [#8249](https://github.com/character-ai/larch/pull/8249) | `f3f4e63dd4a9b4ee79fde0abd490f3a2ea760d26` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31248773154/job/93081635452): `skip` → `full`; `skip-observation-window-open`; `observation_only=true` | [`rust-full`, 346 s](https://github.com/character-ai/larch/actions/runs/31248773154/job/93081688015); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31248773154/job/93082214841); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31248773154/job/93082225084) | false-safe: none observed; false-full: not assessed while `full` was effective |

All three runs used `ubuntu-24.04` with the same Rust-input identity. The
Cargo-inputs, cargo-nextest, and cargo-llvm-cov caches were hits in every run;
the coverage-target cache was deliberately disabled. The comparable full-job
durations were 345 seconds, 352 seconds, and 346 seconds (median 346 seconds).
Each full backstop passed, so this window has zero observed false-safe results
for `skip`. A green full backstop is not false-full evidence while the full lane
is intentionally effective.

The reviewed skip promotion that recorded this window was a global `full`
input, so it did not pretend to supply a selected skip duration. The ordinary
eligible pull request below provides that measurement. The partial class
completed its independent window below.

### Completed partial observation window (2026-08-23)

The complete live artifact history from rollout through promotion contains six
proposed-partial workflow records across four distinct ordinary pull requests.
Pull request #8302 produced three records for separate tested candidates and a
rerun, so it counts once. No retained row was label-forced. The table uses the
latest successful candidate from #8302; every listed full backstop passed.

| PR | Tested merge candidate | Proposed/effective decision | Full backstop | Comparison |
|---|---|---|---|---|
| [#8281](https://github.com/character-ai/larch/pull/8281) | `c9c0ce0a8ca12a5067176b7a5c4e2a3b6611ba0d` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31278658870/job/93156285633): `partial` → `full`; `partial-observation-window-open`; `rollout_state=observation`; `observation_only=true` | [`rust-full`, 391 s](https://github.com/character-ai/larch/actions/runs/31278658870/job/93156350059); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31278658870/job/93157028898); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31278658870/job/93157039215) | false-safe: none observed; false-full: not assessed while `full` was effective |
| [#8287](https://github.com/character-ai/larch/pull/8287) | `537e4b50c4e46a43683375a2f75a4683cc2cdc34` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31280181750/job/93160106052): `partial` → `full`; `partial-observation-window-open`; `rollout_state=observation`; `observation_only=true` | [`rust-full`, 397 s](https://github.com/character-ai/larch/actions/runs/31280181750/job/93160158434); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31280181750/job/93160815687); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31280181750/job/93160830017) | false-safe: none observed; false-full: not assessed while `full` was effective |
| [#8302](https://github.com/character-ai/larch/pull/8302) | `b7393697b6787000116eb2c946b859985a6bd741` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31293686433/job/93196710174): `partial` → `full`; `partial-observation-window-open`; `rollout_state=observation`; `observation_only=true` | [`rust-full`, 276 s](https://github.com/character-ai/larch/actions/runs/31293686433/job/93196746714); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31293686433/job/93197157799); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31293686433/job/93197165868) | false-safe: none observed; false-full: not assessed while `full` was effective |
| [#8380](https://github.com/character-ai/larch/pull/8380) | `676746f63559aa76518f6a88480224fe63f9bc0c` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31461751099/job/93686492399): `partial` → `full`; `partial-observation-window-open`; `rollout_state=observation`; `observation_only=true` | [`rust-full`, 340 s](https://github.com/character-ai/larch/actions/runs/31461751099/job/93687102773); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31461751099/job/93688054757); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31461751099/job/93688074881) | false-safe: none observed; false-full: not assessed while `full` was effective |

All four rows used `ubuntu-24.04`. Cargo-inputs, cargo-nextest, and
cargo-llvm-cov restored exact-key hits in every row. The coverage-target cache
was disabled for #8281 and #8287, whose full jobs took 391 and 397 seconds. It
restored exact-key hits for #8302 and #8380, whose full jobs took 276 and 340
seconds. Every retained full backstop passed, so the partial window has zero
observed false-safe results. A green full backstop is not false-full evidence
while the full lane is intentionally effective.

The bounded-history optimization in #8288 preserved the selector, effective
mode resolution, and fail-closed ancestry proof. The Python-to-Rust cutover
in #8368 proved parity and preserved the same decision contract. The
executable reuse in #8381 retained the exact content-derived trusted-main
identity and routes every restore or validation failure to `full`. These
changes did not weaken the partial decision or its trust contract, so they did
not reset this class-specific safety window. No retained post-#8381 row had yet
exercised the selected partial lane, however; the first enforced run exposed
the false-full liveness defect below.

The reviewed partial promotion is a global `full` input, so it cannot supply a
selected partial duration. The post-promotion evidence below records the first
eligible pull request and must include a succeeding selected-path measurement
and required-status results before it is complete.

### Post-promotion partial liveness correction (2026-08-23)

The first eligible single-crate pull request, [#8873](https://github.com/character-ai/larch/pull/8873),
initially exercised the safe fallback rather than the partial lane. Its
[`rust-ci-selection` artifact](https://github.com/character-ai/larch/actions/runs/32672809881/job/97276118577)
recorded `trusted-main-policy-unavailable-or-invalid`, proposed and effectively
selected `full`, and omitted candidate-derived paths. The paired
[`rust-full`, 716 s](https://github.com/character-ai/larch/actions/runs/32672809881/job/97276192814),
[`rust-coverage`](https://github.com/character-ai/larch/actions/runs/32672809881/job/97277641082),
and [`rust-gate`](https://github.com/character-ai/larch/actions/runs/32672809881/job/97277655959)
jobs succeeded. This is a false-full liveness result, not a false-safe result.

The cache action had derived the policy key and expected digest from the
candidate Rust tree. Any Rust edit therefore named an exact key that only the
unmerged candidate could have published, making partial selection unreachable.
The corrective workflow derives both values from the isolated trusted base by
running that base's cache-key action. It retains the same exact key schema,
checksum, input digest, `refs/heads/main` provenance, source-SHA shape, version,
and fail-to-full validation. Candidate files still cannot supply an executable
or identity. This restores the intended trusted-base lookup without changing
the classifier or weakening its trust contract. Pull request
[#8874](https://github.com/character-ai/larch/pull/8874) carried that correction.

The first selected-partial run after that correction,
[`32676763609`](https://github.com/character-ai/larch/actions/runs/32676763609),
proved the selector and Rust producer but exposed a downstream contract error.
[`rust-partial`, 519 s](https://github.com/character-ai/larch/actions/runs/32676763609/job/97286320990),
[`rust-coverage`](https://github.com/character-ai/larch/actions/runs/32676763609/job/97287516941),
and [`rust-gate`](https://github.com/character-ai/larch/actions/runs/32676763609/job/97287537385)
succeeded while `rust-full` stayed skipped. The required
[`python-tests-gate`](https://github.com/character-ai/larch/actions/runs/32676763609/job/97287537469)
failed because it required LLVM profile output from the deliberately
uninstrumented partial binary. Pull request
[#8875](https://github.com/character-ai/larch/pull/8875) made that assertion
mode-aware while preserving it for coverage-built `full` and `skip` binaries.
Artifact checksum, source, version, and producer validation did not change.

The final same-candidate validation is complete:

| Run | Artifact decision | Selected Rust producer | Required result |
|---|---|---|---|
| [Enforced partial `32679479548`, attempt 2](https://github.com/character-ai/larch/actions/runs/32679479548) | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/32679479548/job/97293497726): `partial` → `partial`; `selector-proposed-partial`; `rollout_state=enforced`; `observation_only=false` | [`rust-partial`, 455 s](https://github.com/character-ai/larch/actions/runs/32679479548/job/97295111599); [`rust-full` skipped](https://github.com/character-ai/larch/actions/runs/32679479548/job/97295112296) | [`rust-coverage`](https://github.com/character-ai/larch/actions/runs/32679479548/job/97296250673), [`rust-gate`](https://github.com/character-ai/larch/actions/runs/32679479548/job/97296275028), and [`python-tests-gate`](https://github.com/character-ai/larch/actions/runs/32679479548/job/97296275041) succeeded; all 13 required contexts passed |
| [Label-forced full `32680683805`](https://github.com/character-ai/larch/actions/runs/32680683805) | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/32680683805/job/97296735447): `partial` → `full`; `forced-by-full-rust-ci-label`; `rollout_state=forced-full`; `observation_only=false` | [`rust-full`, 707 s](https://github.com/character-ai/larch/actions/runs/32680683805/job/97296843971); [`rust-partial` skipped](https://github.com/character-ai/larch/actions/runs/32680683805/job/97296844478) | [`rust-coverage`](https://github.com/character-ai/larch/actions/runs/32680683805/job/97298512788), [`rust-gate`](https://github.com/character-ai/larch/actions/runs/32680683805/job/97298533574), and [`python-tests-gate`](https://github.com/character-ai/larch/actions/runs/32680683805/job/97298533596) succeeded; all 13 required contexts passed |

The first attempt of run `32679479548` stopped safely when an unchanged test
fixture reported `could not read bundle executable version` for its temporary
shell after 1,893 sibling tests passed. The identical failed-job rerun passed
and is recorded separately above; the first attempt is not claimed as
successful evidence. On the successful same-candidate comparison, the partial
producer was 252 seconds (36%) shorter than the forced-full producer. The
label-forced row validates the escape hatch but remains excluded from the live
observation window.

### Live-row collection

Treat each row as evidence from the exact pull-request workflow run, not as a
claim made by its branch. Download that run's `rust-ci-selection` artifact and
record its proposed mode, effective mode, effective-mode reason, and
`observation_only` value. Then record the linked full-mode producer jobs,
the parallel `rust-full LCOV tool` result, and the `rust-coverage` and
`rust-gate` job results and durations from the same run. Record a rerun or
a job from a different merge candidate separately, but do not let it replace
the original row or satisfy the distinct-pull-request requirement. A
label-forced run is not eligible evidence.

### Comparison outcome

For a proposed non-full row, record `false-safe: none observed` only when the
same run's full backstop is green. A full-backstop failure is inconclusive until
its scope is investigated: mark it false-safe when it exposes required work
that the proposed path would omit, and otherwise record the failure without
promoting the class. A green full backstop does not establish a selected-path
duration or turn the row into false-full evidence.

### Timing comparability

Record the runner image, tool and cache class, and the full-mode critical-path
duration for every live row. After a class is enabled, compare its selected-path
duration only with full rows on the same runner image and with the same
Rust-input identity. Distinguish cold from warm runs and explicitly name when
the lanes intentionally use different cache mechanisms, such as Cargo/tool
caches for `full` and a verified trusted-main artifact for `skip`; do not call
those different mechanisms the same cache class. The lightweight aggregate jobs
confirm required dependencies; they do not replace the selected execution-path
duration.

### Enforced skip measurement

| PR | Tested merge candidate | Enforced decision | Selected path and required statuses | Result |
|---|---|---|---|---|
| [#8252](https://github.com/character-ai/larch/pull/8252) | `b98d5e4b2669f79f6f1516ed307afef8c5ad78c4` | [`rust-ci-selection`](https://github.com/character-ai/larch/actions/runs/31249895522/job/93084462058): proposed/effective `skip`; `selector-proposed-skip`; `rollout_state=enforced`; `observation_only=false` | [`rust-skip`, 71 s](https://github.com/character-ai/larch/actions/runs/31249895522/job/93084512640); [`rust-coverage` success](https://github.com/character-ai/larch/actions/runs/31249895522/job/93084626961); [`rust-gate` success](https://github.com/character-ai/larch/actions/runs/31249895522/job/93084645746); [`rust-full` skipped](https://github.com/character-ai/larch/actions/runs/31249895522/job/93084512895) | verified trusted-main artifact, repository policy, and plugin validation all succeeded |

The selected `rust-skip` job completed trusted-main artifact verification,
repository policy, and plugin validation before it succeeded. Its 71-second
duration is the selected execution-path measurement. The required Rust path
from `rust-selection` completion through `rust-gate` completion took 92 seconds.
The comparable full controls were 362 seconds (#8247), 370 seconds (#8248),
and 368 seconds (#8249), for a 368-second median; the enforced skip path is
therefore 276 seconds (75%) shorter on that measured Rust PR critical path.

All four runs used `ubuntu-24.04` and the same Rust-input identity. The full
controls restored warm Cargo-inputs, cargo-nextest, and cargo-llvm-cov caches
with the coverage-target cache disabled. The selected skip lane instead used a
verified trusted-main policy artifact, so this is a class-specific cache
comparison rather than a claim that both lanes restored the same cache. Its
successful repository-policy and plugin-validation step retains the coverage
that makes this skip decision safe.

Do not count a label-forced run, selector failure fallback, or a historical
replay as a live observation. A class may be promoted only if every live row
for that class has zero false-safe results. A false-safe result keeps that class
on `full`; a false-full result may improve the classifier but does not justify
promotion. Any selector, ownership, trusted-binary, cache-schema, or workflow
change that changes selection or the trust contract starts a fresh live window
for the affected class. The reviewed class-specific enforcement toggle that
follows a completed window does not change the classifier or its trusted-input
contract.

## Rust-selection critical-path measurement (2026-08-08)

Issue #8274 compares three successful pull-request controls with three
successful workflow attempts of the same immutable [#8288 pull-request merge
candidate](https://github.com/character-ai/larch/pull/8288). The repeated
attempts make scheduler and full-lane variation visible without changing the
candidate. They are performance samples, not distinct live-observation rows.
Every sample used `ubuntu-24.04`, proposed and ran `full` because this workflow
change is a global input, had the coverage-target cache disabled, and completed
`rust-full`, `rust-coverage`, and `rust-gate` successfully. The trusted-main
policy restore and validation were skipped in every full-path control.

The candidate assessment retained the base worktree: the pre-change controls
showed only 0.21--0.24 seconds for its creation. Moving the trusted-main policy
cache could not improve these full paths because its restore and validation were
already skipped. Starting `rust-full` unconditionally was rejected because it
would either run a second lane for selected `partial` or `skip` paths or weaken
the `rust-coverage` assertion that exactly the selected lane passes. The safe
reduction is a depth-eight checkout, followed by a proof of both commits and
base ancestry. Valid identities with insufficient history fetch complete branch
history and repeat that proof; an invalid identity or failed proof remains an
explicit `full` fallback.

### Depth-two candidate trial

The first candidate used depth two. Its three reruns reduced the selector job
to 8--10 seconds and the serialized prelude to 12--14 seconds, but they were
not the final configuration. A later ordinary validation run on a generated
merge candidate,
[run 31282392390](https://github.com/character-ai/larch/actions/runs/31282392390/job/93165706666),
needed the explicit `full-history-fallback`: its history preparation took
13.156 seconds and its selection job took 22 seconds. That candidate placed the
pull-request base three graph levels below the tested merge candidate, so depth
two remained safe but did not reliably remove the serial checkout cost. Depth
eight covers that observed nested merge shape while retaining the same proof
and full-history fallback.

| Group | Pull-request run | Selection job | Checkout | Trusted history | Base worktree | Selector command | Policy restore / validation | Artifact upload | Prelude to `rust-full` | Selection-to-gate critical path |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: |
| Before | [run 31279643073](https://github.com/character-ai/larch/actions/runs/31279643073/job/93158778455) | 19 s | 14 s | full checkout | 0.24 s | 0.62 s remainder | skipped / skipped | 1 s | 22 s | 424 s |
| Before | [run 31278658870](https://github.com/character-ai/larch/actions/runs/31278658870/job/93156285633) | 29 s | 14 s | full checkout | 0.21 s | 8.50 s remainder | skipped / skipped | 1 s | 33 s | 436 s |
| Before | [run 31276770466](https://github.com/character-ai/larch/actions/runs/31276770466/job/93151508673) | 23 s | 16 s | full checkout | 0.23 s | 0.63 s remainder | skipped / skipped | 1 s | 27 s | 398 s |
| Before median | three controls | 23 s | 14 s | full checkout | 0.23 s | 0.63 s remainder | skipped / skipped | 1 s | 27 s | 424 s |
| After depth 8 | [#8288, attempt 1](https://github.com/character-ai/larch/actions/runs/31282787491/job/93166673167) | 10 s | 2 s | 0.007 s, `bounded-depth-8` | 0.227 s | 0.117 s | skipped / skipped | 1 s | 15 s | 441 s |
| After depth 8 | [#8288, attempt 2](https://github.com/character-ai/larch/actions/runs/31282787491/job/93167602262) | 7 s | 2 s | 0.007 s, `bounded-depth-8` | 0.228 s | 0.109 s | skipped / skipped | 1 s | 10 s | 408 s |
| After depth 8 | [#8288, attempt 3](https://github.com/character-ai/larch/actions/runs/31282787491/job/93168338371) | 9 s | 2 s | 0.006 s, `bounded-depth-8` | 0.222 s | 0.105 s | skipped / skipped | 1 s | 12 s | 410 s |
| After depth 8 median | three attempts | 9 s | 2 s | 0.007 s, `bounded-depth-8` | 0.227 s | 0.109 s | skipped / skipped | 1 s | 12 s | 410 s |

`Prelude to rust-full` measures from `rust-selection` start to `rust-full`
start; the critical path measures from `rust-selection` start to `rust-gate`
completion. GitHub's job and step timestamps are whole seconds; the history,
worktree, and selector-command entries are the new millisecond measurements.
Before the change, the combined selector step exposed only the remainder after
worktree creation.

Every final depth-eight sample used `bounded-depth-8`; none needed the
full-history fallback. The median serialized prelude fell from 27 to 12 seconds,
while selection job duration fell from 23 to 9 seconds. The three-sample
end-to-end median changed from 424 to 410 seconds. Full Rust lane variation
still dominates this aggregate, so the difference does not support another
broad topology change. The bounded checkout is retained because it removes a
measured serial cost while preserving trusted-base execution, artifact upload,
and fail-closed behavior.

## Trusted selector executable reuse (2026-08-10)

The Rust CLI cutover in [#8368](https://github.com/character-ai/larch/pull/8368)
introduced a new serialized cost: `rust-selection` built the complete
pull-request-base `larch-cli` before any Rust producer could start. Three
successful pull-request runs show the regression:

| Run | `rust-selection` | Trusted selector build | `rust-full` | Trigger to last job |
| --- | ---: | ---: | ---: | ---: |
| [31448285158](https://github.com/character-ai/larch/actions/runs/31448285158) | 219 s | 196.462 s | 433 s | 764 s |
| [31449591839](https://github.com/character-ai/larch/actions/runs/31449591839) | 227 s | 202.441 s | 425 s | 703 s |
| [31451833760](https://github.com/character-ai/larch/actions/runs/31451833760) | 217 s | cold full-CLI build | 344 s | 612 s |

A comparable successful run before that cutover, [31448985417](https://github.com/character-ai/larch/actions/runs/31448985417),
completed selection in 11 seconds and `rust-full` in 318 seconds. The later
full-lane samples also exposed exact dependency-cache misses, but those misses
do not justify a broad restore fallback: prior measurements showed that the
fallback transferred a large incompatible target without avoiding a cold
build.

Issue [#8378](https://github.com/character-ai/larch/issues/8378) removes only
the selector build. Selection restores and validates the existing exact
`trusted-main-rust-policy` executable before invoking the pull-request-base
wrapper. A miss, invalid artifact, or unavailable trusted-base Rust-input
identity produces a static `full` decision. The producer dependency remains
intentional: starting
`rust-full` before selection would either waste a full run on a selected
`partial` or `skip` path, or weaken the exact-one-producer assertion in
`rust-coverage`.

The first live implementation run confirmed that the compiled selector was the
serialized regression:

| Run | `rust-selection` | Exact policy restore | Selector command | Policy artifact upload | Prelude to `rust-full` |
| --- | ---: | ---: | ---: | ---: | ---: |
| [#8381, initial implementation](https://github.com/character-ai/larch/actions/runs/31462693997/job/93689229788) | 36 s | hit, 3 s | 0.185 s | 6 s | 39 s |

The three regressed controls have a 219-second median selection duration, so
the initial implementation removed 183 seconds (84%). The immediately prior
[#8380 run](https://github.com/character-ai/larch/actions/runs/31461751099)
spent 222 seconds from selection start to `rust-full` start; #8381 spent 39
seconds, a reduction of 183 seconds (82%). The initial run still uploaded the
44 MB verified policy handoff on its selected `full` path. The final workflow
uploads that artifact only for an effective `skip`, its sole consumer, so full
and partial decisions avoid another measured six seconds of serialized work.
The same run's `rust-full` lane passed in 272 seconds, then `rust-coverage` and
`rust-gate` passed; the complete workflow took 350 seconds from trigger through
the last job.

## Timing interpretation and rollback

The historical full `rust-coverage` samples above are contextual baselines: the
partial rows have a median of 140 seconds and the skip rows a median of 205
seconds. They are not timings for non-full jobs. The completed live skip window
adds a comparable pre-enforcement full-job control with a median of 346 seconds.
Only after a class is enabled can its `rust-partial` or `rust-skip` duration
demonstrate a critical-path reduction against comparable full-backstop samples.

To roll back selection immediately, apply the `full-rust-ci` label to a pull
request. To roll back a decision class permanently, keep its enforcement value
`false`, remove its audited owner from `crates/larch-cli/src/ci_selection.rs`,
or route its path to a global `full` trigger. A cache miss, schema change,
input-identity mismatch, checksum failure, or provenance failure already falls
back to `full` without an operator action.
