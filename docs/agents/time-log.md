# Time Log

`docs/time-log.md` records a rough estimate of the time spent on this repo, one row per day.
`Active` is the time the human and the agent worked together; `AFK` is the time an agent ran unattended.
Update it only when the human asks, for example "log time" or "Zeit loggen".

## Source

The Claude Code transcripts are the source: every `*.jsonl` file directly inside each directory under `~/.claude/projects/` whose name contains `hoof-and-claw`.
That match covers the main checkout and every worktree.
A session's subagent transcripts sit in `<session-id>/subagents/*.jsonl` next to its file.
Claude Code deletes transcripts after 30 days by default, so a day older than that may have no transcripts left.

## AFK sessions

An AFK session is one where the human invoked the `kipu-herd` or `kipu-ship` skill: a `user` line's text content, not a tool result, holds a `<command-name>` that ends in `kipu-herd` or `kipu-ship`, such as `/wuddi:kipu-herd`.
Every other session is attended.

## Time-log sessions

A time-log session is one whose first human prompt asks to log or measure the time spent on this repo, such as "log time" or a prompt that references this file.
It counts in neither measure and not in `Sessions`, because that time goes into the log, not into the repo.

## Method

1. Leave out every time-log session.
2. For `Active`, collect the `timestamp` of every line in an attended session's file whose `type` is `user` or `assistant` and whose `isSidechain` is not `true`; subagent lines are out.
3. For `AFK`, collect the same timestamps from each AFK session's file, plus the `timestamp` of every `user` or `assistant` line in its subagent transcripts.
4. Pool the timestamps of each measure across its sessions and sort them, so overlapping sessions count once within a measure.
5. Sum each gap between consecutive timestamps of 15 minutes or less; a longer gap is a pause and counts zero.
6. Assign time to days in `Europe/Berlin` time, splitting a gap that crosses midnight.
7. Count a day's sessions as the attended sessions' files with at least one timestamp on that day.

Compute it with a one-off command and leave no script in the repo.

## Updating the log

1. Recompute every day the transcripts still cover and replace its row.
2. Keep each row whose day the transcripts no longer cover exactly as it is; a row from before the `AFK` column shows `–` there.
3. Sort the rows by date, round each time to 5 minutes as `5 h 45 min`, and recompute the total row from the rounded rows.
4. Commit `docs/time-log.md` on `main` as `docs: log time spent` and push.
