# Time Log

`docs/time-log.md` records a rough estimate of the active time the human and the agent spent together on this repo, one row per day.
Update it only when the human asks, for example "log time" or "Zeit loggen".

## Source

The Claude Code transcripts are the source: every `*.jsonl` file directly inside each directory under `~/.claude/projects/` whose name contains `hoof-and-claw`.
That match covers the main checkout and every worktree.
Claude Code deletes transcripts after 30 days by default, so a day older than that may have no transcripts left.

## Method

1. Collect the `timestamp` of every line whose `type` is `user` or `assistant` and whose `isSidechain` is not `true`; subagent lines are out.
2. Pool the timestamps of all sessions and sort them, so overlapping sessions count once.
3. Sum each gap between consecutive timestamps of 15 minutes or less; a longer gap is a pause and counts zero.
4. Assign time to days in `Europe/Berlin` time, splitting a gap that crosses midnight.
5. Count a day's sessions as the transcript files with at least one timestamp on that day.

Compute it with a one-off command and leave no script in the repo.

## Updating the log

1. Recompute every day the transcripts still cover and replace its row.
2. Keep each row whose day the transcripts no longer cover exactly as it is.
3. Sort the rows by date, round each time to 5 minutes as `5 h 45 min`, and recompute the total row from the rounded rows.
4. Commit `docs/time-log.md` on `main` as `docs: log time spent` and push.
