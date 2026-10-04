# Step Prefix Encoding

**Consumer**: parent orchestrators and child skills that accept nested step context.

**Contract**: nested `--step-prefix` encoding and parsing only.

**When to load**: only when documenting or invoking nested parent-to-child skill dispatch.

## `--step-prefix` Encoding

When a parent skill invokes a child skill (e.g., `/implement` → `/design`), it passes step context via `--step-prefix` using this encoding:

```
--step-prefix "NUM_PREFIX::TEXT_PATH::PARENT_SKILL_PATH"
```

- **`NUM_PREFIX`**: The numeric prefix to prepend to the child's step numbers (e.g., `"1."` means child step `2a` becomes `1.2a`).
- **`TEXT_PATH`**: The human-readable breadcrumb segment(s) from the parent (e.g., `"design plan"`).
- **`PARENT_SKILL_PATH`**: Optional slash-prefixed parent skill path (e.g., `"/implement"`). The child appends its own slash-prefixed skill name with `:` as the separator.
- **Delimiter**: Split on the first two `::` delimiters to separate numeric prefix, textual breadcrumb path, and optional parent skill path.
- **Backward compatibility**: If `::` is absent, treat the entire value as a numeric-only prefix. The text path defaults to empty — breadcrumbs show only the leaf step name.

### Parsing in child skills

Child skills parse `--step-prefix` into three mental variables:

- `STEP_NUM_PREFIX`: Everything before the first `::` (or the entire value if `::` absent).
- `STEP_PATH_PREFIX`: Everything after the first `::` and before the second `::` (or empty if absent).
- `PARENT_SKILL_PATH`: Everything after the second `::` (or empty if absent).

When outputting a step:

- **Skill path**: Standalone uses the local skill name, e.g., `/design`. Nested uses `{PARENT_SKILL_PATH}:/{local_skill_name}` when `PARENT_SKILL_PATH` is non-empty.
- **Step number**: `{STEP_NUM_PREFIX}{local_step_number}` (e.g., `1.` + `2b.5` = `1.2b.5`)
- **Breadcrumb path**: If `STEP_PATH_PREFIX` is non-empty: `{STEP_PATH_PREFIX} | {step_short_name}`. Otherwise: just `{step_short_name}`.

### Examples

Standalone `/design` (no `--step-prefix`):
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /design 2a: sentinel prep**
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /design 2b: full plan**
```

`/design` called from `/implement` with `--step-prefix "1.::design plan::/implement"`:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /implement:/design 1.2a: design plan | sentinel prep**
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /implement:/design 1.2b: design plan | full plan**
```

`/review` called from `/implement` with `--step-prefix "5.::code review::/implement"`:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /implement:/review 5.2: code review | launch reviewers**
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /implement:/review 5.3: code review | review cycle**
```
