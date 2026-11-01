# Time log

This log is a rough estimate of the time per day, in Berlin time, that went into this repo through Claude Code sessions.
`Active` is time the human and the agent worked together; `AFK` is time an agent ran unattended through `kipu-herd`.
Pauses over 15 minutes count zero, parallel sessions count once, and sessions spent on this log count zero.
The agent updates this file on request as described in `docs/agents/time-log.md`.

Most of this time went into the groundwork a production service needs before its first feature: 15 ADRs, the architecture and coding standards, a Green gate with acceptance tests and performance budgets, CI, infrastructure, and a verification loop in which agents prove their own changes.
About half of the commits are `docs` commits.
Later features reuse this groundwork instead of building it again.
`Active` also counts time in which the agent worked while the human watched or reviewed.

| Date       | Active         | AFK            | Sessions |
| ---------- | -------------- | -------------- | -------- |
| 2026-10-30 | 0 h 15 min     | 0 h 00 min     | 3        |
| 2026-10-31 | 6 h 15 min     | 2 h 00 min     | 25       |
| 2026-11-01 | 2 h 55 min     | 0 h 55 min     | 16       |
| **Total**  | **9 h 25 min** | **2 h 55 min** |          |
