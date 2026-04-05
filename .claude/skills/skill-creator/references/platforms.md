# Platform-Specific Adaptations

Read this file if you detect you are running in Claude.ai or Cowork rather than Claude Code. These overrides take priority over the default workflow described in SKILL.md.

## Claude.ai

The core workflow is the same (draft -> test -> review -> improve -> repeat), but Claude.ai does not have subagents. Adapt as follows:

**Running test cases**: No subagents means no parallel execution. For each test case, read the skill's SKILL.md, then follow its instructions to accomplish the test prompt yourself. Do them one at a time. This is less rigorous than independent subagents, but the human review step compensates. Skip the baseline runs — just use the skill to complete the task as requested.

**Reviewing results**: If you cannot open a browser, skip the browser reviewer. Instead, present results directly in the conversation. For each test case, show the prompt and the output. If the output is a file the user needs to see (like a .docx or .xlsx), save it to the filesystem and tell them where it is so they can download and inspect it. Ask for feedback inline.

**Benchmarking**: Skip the quantitative benchmarking — it relies on baseline comparisons which are not meaningful without subagents. Focus on qualitative feedback from the user.

**The iteration loop**: Same as before — improve the skill, rerun the test cases, ask for feedback — just without the browser reviewer in the middle. You can still organize results into iteration directories on the filesystem.

**Description optimization**: This requires the `claude` CLI tool (specifically `claude -p`) which is only available in Claude Code. Skip it on Claude.ai.

**Blind comparison**: Requires subagents. Skip it.

**Packaging**: The `package_skill.py` script works anywhere with Python and a filesystem. On Claude.ai, run it and the user can download the resulting `.skill` file.

**Updating an existing skill**: Preserve the original name and `name` frontmatter field. Copy to a writeable location before editing (the installed skill path may be read-only). If packaging manually, stage in `/tmp/` first, then copy to the output directory.

## Cowork

In Cowork, the main things to know:

- You have subagents, so the main workflow (spawn test cases in parallel, run baselines, grade, etc.) all works. If you run into severe timeout problems, run the test prompts in series rather than parallel.
- You do not have a browser or display. When generating the eval viewer, use `--static <output_path>` to write a standalone HTML file instead of starting a server. Then provide a link the user can click to open the HTML in their browser.
- Generate the eval viewer using `generate_review.py` BEFORE evaluating outputs yourself. Get results in front of the human as soon as possible.
- Feedback works differently: since there is no running server, the viewer's "Submit All Reviews" button will download `feedback.json` as a file. Read it from there (you may need to request access first).
- Packaging works — `package_skill.py` just needs Python and a filesystem.
- Description optimization (`run_loop.py` / `run_eval.py`) works in Cowork since it uses `claude -p` via subprocess, but save it until you have fully finished making the skill and the user agrees it is in good shape.
- **Updating an existing skill**: Follow the same update guidance as the Claude.ai section above.
