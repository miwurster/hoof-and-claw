# Free-tier hosting and data stores for an always-on game server

Research for [#3](https://github.com/miwurster/hoof-and-claw/issues/3), part of the map [#1](https://github.com/miwurster/hoof-and-claw/issues/1).
All sources were read on 2026-10-31 unless a line says otherwise.
Terms follow `GLOSSARY.md`.

## Question

Which free tiers can run the Hoof & Claw server always on, with no visible cold start, and resolve an Open guess on a timer while no browser is open?

The constraints from #1 that decide the answer:

- Free tiers only, no own AWS account; a tier that bills only past a limit we will not reach is acceptable.
- No visible cold start for a reviewer.
- A Guess resolves on the server about 60 seconds after placement, once the price has changed, with no browser open.
- The demo runs at least six months without maintenance.
- Infrastructure as code is wanted.
- Rust is the favourite server language, TypeScript the alternative; the frontend is React.

## Answer in short

Only two kinds of free offer meet all hard constraints: Cloudflare Workers with Durable Objects (serverless, alarms per object, WebSockets with hibernation, Rust via `workers-rs`) and an always-free virtual machine (Oracle Cloud Always Free or Google Cloud's `e2-micro`).
Fly.io, Render and Railway no longer have a free tier that can run an always-on server.
Deno Deploy is being shut down after Deno joined Cloudflare.
Vercel Hobby cannot run a timer more often than once a day and cannot hold a WebSocket.
Supabase, Neon, Turso and Upstash are useful as data stores or schedulers, but none is a full answer on its own, and Supabase and Neon pause or suspend when idle.

## Comparison

"Verified" cells come from the cited source; cells marked _(inference)_ are reasoning on top of the sources.

| Option | Free limits (key ones) | Past the limit | Card needed | Cold start | Timer with no browser | WebSocket / SSE | Rust | IaC | Region |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Cloudflare Workers + Durable Objects | Workers: 100,000 requests/day, 10 ms CPU per request, 5 Cron Triggers. DO: 100,000 requests/day, 13,000 GB-s/day, 5 M rows read/day, 100,000 rows written/day, 5 GB SQLite | Requests fail with Error 1027; DO operations of that type fail with an error until 00:00 UTC; no bill | No _(inference: none mentioned on the pricing pages)_ | Isolates, no VM cold start | DO alarms, at-least-once, retried; Cron Triggers (min 1 minute) | WebSockets on DO with Hibernation API | Yes, `workers-rs` v0.8.7 (2026-09-25) supports DO, SQLite, alarms, WebSocket accept, scheduled events | Terraform `cloudflare/cloudflare` 5.28.1 (2026-10-20); Wrangler | Global edge; DO placed near first caller |
| Fly.io | None for new organizations since October 2024; trial of 2 machine-hours or 7 days | Card required to continue; smallest always-on machine $2.19 per 30 days | Yes after trial | None if a machine stays running | Any (it is a VM) | Yes | Yes | Terraform provider `fly-apps/fly` last released 2023-06-22 _(stale)_ | Many regions |
| Deno Deploy | 1 M requests/month, 10 h active CPU, 1 GiB KV, Cron | Not stated for Free | Not stated | Idle apps shut down after about 20 to 30 s | `Deno.cron` | Not stated in sources read | No (JS/TS only) | No current provider (`denoland/deno` 0.1.0 from 2023) | 2 regions |
| Vercel Hobby + Upstash | Vercel: 1 M function invocations, 4 active CPU-hours, cron once per day (±59 min). Upstash Redis: 500 K commands/month, 250 MB. QStash: 1,000 messages/day, delay up to 7 days | Vercel: feature blocked until 30 days pass. QStash: soft limit, then HTTP 429 | Not for Hobby _(inference)_ | Serverless functions; idle cold starts possible _(inference)_ | Not with Vercel Cron on Hobby; a QStash delayed message per Guess could call a function | Functions cap at 300 s, so no long-lived WebSocket _(inference)_ | Rust on Vercel Functions not checked | Terraform `vercel/vercel` 5.24.1 (partner), `upstash/upstash` 2.1.0 (community) | Function region of choice |
| Vercel Daemons (beta) | Hobby: 5 active CPU-hours and 420 GB-hours provisioned memory per month across the team | All daemon instances stop until the 1st of the next month | Not for Hobby _(inference)_ | Always running while quota lasts | Any (container) | No inbound traffic | Any (Linux amd64 container) | Same as Vercel | Configurable |
| Supabase Free | 500 MB Postgres, 500,000 Edge Function invocations, 200 Realtime connections, 2 M Realtime messages, 5 GB egress, 2 projects | Not stated for Free | Not stated | Edge Functions are serverless _(inference)_ | `pg_cron`, every second to once a year, can call an Edge Function over HTTP | Realtime (WebSocket) | No; Edge Functions run TypeScript on Deno _(inference)_ | Terraform `supabase/supabase` 1.11.0 (community) | Region per project |
| Neon Free (data store only) | 100 CU-hours per project per month, 1 GB storage per project, 100 projects, 5 GB egress | Compute suspended until next period; writes blocked past storage | No | Scale to zero after 5 min, cannot be turned off | None (data store) | n/a | Postgres from any language | Terraform `kislerdm/neon` 0.18.0 (community) | Region per project |
| Turso Free (data store only) | 100 databases, 5 GB, 500 M rows read and 10 M rows written per month | Not stated for Free | No | Not stated | None (data store) | n/a | `libsql` client _(inference)_ | No provider found in the Terraform registry under `tursodatabase/turso` | Not checked |
| Oracle Cloud Always Free | Ampere A1: 1,500 OCPU-hours and 9,000 GB-hours a month, equal to 2 OCPU and 12 GB memory; 2 AMD micro VMs (1/8 OCPU, 1 GB); 200 GB block storage; 2 Autonomous DBs (20 GB); 10 TB egress/month | Always Free resources stay after the trial; paid resources are reclaimed | Yes (and phone) | None, always-on VM | Any (it is a VM) | Yes | Yes | Terraform `oracle/oci` 9.11.0 (partner) | Home region only, chosen at sign-up |
| Google Cloud Free Tier `e2-micro` | 1 `e2-micro` VM, 30 GB disk, 1 GB egress/month from North America; Firestore 50,000 reads and 20,000 writes per day; Cloud Run 2 M requests/month | Billed past the limit _(inference: Free Tier needs a billing account)_ | Yes (billing account) | None for the VM | Any (it is a VM) | Yes | Yes | Terraform `hashicorp/google` _(not checked)_ | `us-west1`, `us-central1` or `us-east1` only |
| Render Free | 750 instance hours/month per workspace | n/a | Not checked | Spins down after 15 min idle, about 1 min to spin up | No; cron jobs and background workers not available on Free | Yes while awake | Yes | Not checked | Not checked |
| Railway | $5 one-time trial, then $1 credit per month | n/a | Not checked | n/a | n/a | n/a | n/a | n/a | n/a |

## Findings per option

### Cloudflare Workers with Durable Objects

Verified:

- The Workers Free plan allows 100,000 requests a day, resetting at midnight UTC; past that, Cloudflare returns Error 1027, and the route's fail mode decides whether the Worker is bypassed or an error page is shown ([Workers limits](https://developers.cloudflare.com/workers/platform/limits/)).
- CPU time is 10 ms per HTTP request and per Cron Trigger invocation; waiting on network calls does not count ([Workers limits](https://developers.cloudflare.com/workers/platform/limits/)).
- An account on the Free plan can have 5 Cron Triggers ([Workers limits](https://developers.cloudflare.com/workers/platform/limits/)).
- The Free plan can only create SQLite-backed Durable Objects, with 100,000 requests, 13,000 GB-s duration, 5 million rows read and 100,000 rows written a day, and 5 GB stored in total ([DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)).
- Past any one Free limit, further operations of that type fail with an error; the daily limits reset at 00:00 UTC ([DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)).
- Each `setAlarm()` is billed as one row written; on the Paid plan alarm invocations count as requests; the page does not say how alarms count on Free ([DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)).
- A WebSocket connection counts as one request, incoming messages count 20:1, and outgoing messages are free ([DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)).
- With the Hibernation API, clients stay connected while the object hibernates, and duration does not accrue during hibernation ([DO WebSockets](https://developers.cloudflare.com/durable-objects/best-practices/websockets/)).
- Each object has one alarm at a time; alarms have at-least-once execution, retry with exponential backoff from 2 s up to 6 times, and complete "without relying on incoming requests to keep the Durable Object alive" ([DO alarms](https://developers.cloudflare.com/durable-objects/api/alarms/)).
- Isolates start "around a hundred times faster than a Node process" and eliminate "the cold starts of the virtual machine model" ([How Workers works](https://developers.cloudflare.com/workers/reference/how-workers-works/)).
- Requests to static assets are "free and unlimited" ([Static assets billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/)); the page does not say whether they count toward the 100,000 daily requests.
- The `workers-rs` repository has code for Durable Objects with SQLite, `set_alarm`, `accept_web_socket` and scheduled events, and its latest release is v0.8.7 of 2026-09-25 ([workers-rs](https://github.com/cloudflare/workers-rs), [releases](https://github.com/cloudflare/workers-rs/releases)).
- The README warns of "a few rough edges, some unimplemented APIs", and Tokio is not supported ([workers-rs](https://github.com/cloudflare/workers-rs)).
- The Cloudflare Terraform provider is at 5.28.1, published 2026-10-20 ([Terraform registry](https://registry.terraform.io/providers/cloudflare/cloudflare/latest)).

Inferences:

- A design with one alarm per Open guess fits the Free plan at demo scale: placing a Guess costs about one Worker request, one DO request and one row written for the alarm, and each re-check while the price has not changed costs one more alarm.
- A design that polls the price every second on an alarm loop does not fit: 86,400 alarms a day means 86,400 rows written a day for `setAlarm()` alone, close to the 100,000 limit, before any Guess is stored.
- Clients that poll the server over HTTP every few seconds would exhaust the 100,000 requests a day with a handful of players; WebSockets with hibernation are the way to push prices and outcomes.
- One Durable Object kept active all day uses 86,400 seconds times 0.128 GB, about 11,000 GB-s, which is below the 13,000 GB-s a day.
  This assumes Cloudflare bills duration at 128 MB per object, a figure the pages read did not confirm.
- The free limits fail closed, never bill, and reset daily, which suits a six-month unattended demo; the risk is an outage until 00:00 UTC if someone floods the game, so abuse protection matters here.

### Fly.io

Verified:

- "New organizations don't have a free tier or a monthly free usage allowance"; organizations on plans discontinued in October 2024 keep their old allowances ([Fly.io pricing](https://docs.fly.io/about/pricing)).
- New accounts get a trial of "up to 2 hours of Machine runtime or 7 days, whichever comes first", without a card; after that a card is required ([Fly.io pricing](https://docs.fly.io/about/pricing)).
- The smallest always-on machine (shared-cpu-1x, 256 MB) costs $2.19 per 30 days in `iad` or `ewr` ([Fly.io pricing](https://docs.fly.io/about/pricing)).
- The Terraform provider `fly-apps/fly` was last released on 2023-06-22 ([Terraform registry](https://registry.terraform.io/providers/fly-apps/fly/latest)).

Inference: Fly.io is out under the free-tier constraint, but it is the cheapest paid fallback for a plain Rust server.

### Deno Deploy

Verified:

- The Free plan offers 1 M requests a month, 10 h active CPU, 1 GiB KV and 1 M KV reads and 500,000 KV writes a month, and idle apps shut down after about 20 to 30 seconds ([Deno Deploy pricing](https://deno.com/deploy/pricing)).
- Deno Deploy Classic shut down on 2026-07-20; the new Deno Deploy runs in 2 regions and has Cron but no Queues ([Deno Deploy docs](https://docs.deno.com/deploy/)).
- On 2026-10-09 Deno announced it is joining Cloudflare and that "Deno Deploy will continue operating for six months before shutting down" ([Deno blog](https://deno.com/blog/cloudflare)).

Inference: Deno Deploy is out, because it shuts down around April 2027, inside the six-month demo window.

### Vercel with Upstash

Verified:

- Hobby includes 1 M function invocations, 4 active CPU-hours and 360 GB-hours of provisioned memory a month, and functions run at most 300 s ([Vercel Hobby](https://vercel.com/docs/plans/hobby), last updated 2026-09-14).
- Past a Hobby limit "you will have to wait until 30 days have passed before you can use the feature again" ([Vercel Hobby](https://vercel.com/docs/plans/hobby)).
- Hobby is for "non-commercial, personal use only" ([Vercel Hobby](https://vercel.com/docs/plans/hobby)).
- Hobby cron jobs run at most once a day with per-hour precision; more frequent expressions fail at deploy ([Vercel cron pricing](https://vercel.com/docs/cron-jobs/usage-and-pricing), last updated 2026-07-15).
- Vercel Daemons, in beta on all plans, run containers continuously, but on Hobby they share 5 active CPU-hours and 420 GB-hours a month, all instances stop when a quota is reached, and they accept no inbound traffic ([Vercel Daemons guide](https://vercel.com/kb/guide/vercel-daemons), published 2026-10-15).
- Upstash Redis Free has 500 K commands a month, 250 MB and one database ([Upstash Redis pricing](https://upstash.com/pricing/redis)).
- QStash Free has 1,000 messages a day as a soft limit, a delay of up to 7 days, and 10 schedules; sustained overuse can lead to HTTP 429 ([QStash pricing](https://upstash.com/pricing/qstash)).

Inferences:

- Vercel Cron cannot resolve Guesses on Hobby; a QStash message delayed by 60 s per Guess could call a Vercel function instead, which caps the demo at about 1,000 Guesses a day.
- A 2 GiB daemon running all month needs about 1,440 GB-hours, so the 420 GB-hour Hobby quota runs out after about nine days; Vercel Daemons are no always-on option on Hobby.
- Without long-lived connections, live prices would need polling or short SSE streams, which costs invocations.

### Supabase

Verified:

- The Free plan has 500 MB database per project, 500,000 Edge Function invocations, 200 peak Realtime connections, 2 M Realtime messages and 5 GB egress, and two active projects ([Supabase pricing](https://supabase.com/pricing)).
- "Free projects are paused after 1 week of inactivity" ([Supabase pricing](https://supabase.com/pricing)).
- Supabase Cron runs jobs "anywhere from every second to once a year" and can invoke an Edge Function over HTTP ([Supabase Cron](https://supabase.com/docs/guides/cron)); the page does not name plan limits.
- The Terraform provider `supabase/supabase` is at 1.11.0 (2026-09-02) ([Terraform registry](https://registry.terraform.io/providers/supabase/supabase/latest)).

Inferences:

- `pg_cron` inside Postgres can resolve Guesses with no browser open, as SQL or by calling an Edge Function, so Supabase alone can be a whole backend in TypeScript or SQL, but not in Rust.
- What counts as "inactivity" is not defined on the pages read; a demo nobody visits for a week may pause even with a cron job running, which breaks the six-month unattended run unless confirmed otherwise.

### Neon and Turso (data stores only)

Verified:

- Neon Free has 100 CU-hours per project a month, 1 GB storage per project and 100 projects, needs no card, and scales to zero after 5 minutes, which "cannot be turned off" on Free ([Neon pricing](https://neon.com/pricing)).
- Running out of CU-hours or egress suspends compute until the next period; exceeding storage blocks writes ([Neon pricing](https://neon.com/pricing)).
- Turso Free has 100 databases, 5 GB, 500 M rows read and 10 M rows written a month, and needs no card ([Turso pricing](https://turso.tech/pricing)).

Inference: a resolver that queries Neon every minute keeps it awake and uses about 730 CU-hours a month at the smallest size, far above 100; Neon fits only a server that touches the database rarely, so Turso or the platform's own store is the better partner for a timer-driven server.

### Oracle Cloud Always Free

Verified:

- Always Free includes VM.Standard.A1.Flex up to 2 OCPUs and 12 GB memory, two VM.Standard.E2.1.Micro instances, 200 GB block storage, two Autonomous Databases of 20 GB, and 10 TB outbound data a month ([OCI Always Free resources](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)).
- Always Free compute must run in the tenancy's home region ([OCI Always Free resources](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)).
- Oracle may reclaim an Always Free instance that is idle for 7 days: 95th-percentile CPU below 20 %, network below 20 %, and memory below 20 % for A1 ([OCI Always Free resources](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)).
- "Most users need a mobile phone number and a credit card to create an account"; the card is not charged unless the account is upgraded ([OCI Free Tier](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier.htm)).
- Always Free resources stay available after the 30-day trial ([OCI Free Tier](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier.htm)).
- The Terraform provider `oracle/oci` is at 9.11.0 (2026-10-21) ([Terraform registry](https://registry.terraform.io/providers/oracle/oci/latest)).

Inferences:

- A small Rust server for tens of players stays far below 20 % CPU, so the idle rule is a real risk for an unattended six-month demo; upgrading to Pay As You Go is commonly reported to exempt the account, but that was not confirmed on an Oracle page.
- A VM means we own patching, TLS, restarts and backups ourselves, which is maintenance the six-month constraint wants to avoid.

### Google Cloud Free Tier

Verified:

- The Free Tier includes one non-preemptible `e2-micro` VM a month in `us-west1`, `us-central1` or `us-east1`, 30 GB-months of standard disk and 1 GB outbound transfer from North America a month ([GCP Free Tier](https://docs.cloud.google.com/free/docs/free-cloud-features)).
- Firestore Free has 1 GiB, 50,000 reads, 20,000 writes and 20,000 deletes a day; Cloud Run has 2 M requests, 360,000 GB-s and 180,000 vCPU-s a month ([GCP Free Tier](https://docs.cloud.google.com/free/docs/free-cloud-features)).
- "A Google Cloud billing account is required to access the Google Cloud Free Tier" ([GCP Free Tier](https://docs.cloud.google.com/free/docs/free-cloud-features)).

Inference: usage past the free amounts is billed to the card, so the 1 GB egress is the limit to watch with WebSockets pushing prices; the latency from US regions to Europe is visible but not a cold start.

### Render and Railway

Verified:

- Render free web services spin down after 15 minutes without inbound traffic and take about one minute to spin up; free Postgres expires after 30 days; cron jobs and background workers have no free instances ([Render free](https://render.com/docs/free)).
- Railway has a 30-day trial with a one-time $5 grant, then a Free plan with $1 of credit a month ([Railway free trial](https://docs.railway.com/reference/pricing/free-trial)).

Inference: neither runs an always-on server for free.

## Frontend hosting next to each option

- Cloudflare: Workers static assets in the same Worker, free and unlimited ([Static assets billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/)).
- Vercel Hobby: the React build on Vercel's CDN, 1 M CDN requests and 100 GB fast data transfer a month, non-commercial use only ([Vercel Hobby](https://vercel.com/docs/plans/hobby)).
- Supabase, Neon, Turso, Oracle, Google Cloud: any static host works; Cloudflare static assets is the natural free choice, and the VM options can also serve the build themselves.

## Shortlist

1. **Cloudflare Workers + Durable Objects (SQLite) + Workers static assets, server in Rust via `workers-rs`.**
   It is the only option that is always on without a VM, has no cold start, has per-object alarms that fire with no client, holds WebSockets with hibernation, fails closed instead of billing, and has a current Terraform provider.
   The cost is a design that must stay within 100,000 requests and 100,000 rows written a day: one alarm per Guess, no per-second price polling loop on the server, and WebSockets instead of HTTP polling.
   It also ties Rust to `workers-rs` (no Tokio, some unimplemented APIs) or pushes the server to TypeScript, where the Workers platform is first-class.
2. **Oracle Cloud Always Free A1 VM + SQLite or Postgres on its block volume + Cloudflare static assets, server in Rust with Tokio.**
   It gives the plainest Rust server and the most headroom (2 OCPU, 12 GB, 10 TB egress), with a current Terraform provider.
   The cost is the idle-reclamation rule, a card at sign-up, a single home region, possible capacity shortages, and OS maintenance over six months.
3. **Google Cloud `e2-micro` + SQLite on its disk (or Firestore) + Cloudflare static assets, server in Rust.**
   The same plain-VM model as Oracle with a smaller machine (1 GB memory), US regions only, and 1 GB free egress a month, past which usage is billed.

Not shortlisted: Fly.io (no free tier), Deno Deploy (shutting down around April 2027), Vercel Hobby (cron once a day, no WebSocket, daemons quota lasts about nine days), Render and Railway (no always-on free compute), Supabase (pauses after a week of inactivity, no Rust), Neon (scales to zero, compute hours too few for a timer).

## Deciding trade-offs

- Serverless edge (Cloudflare) against plain VM (Oracle, Google): no maintenance and no cold start against full Rust and Tokio freedom with OS upkeep and reclamation risk.
- Rust on Workers means `workers-rs` and WebAssembly; the domain core can stay plain Rust behind ports, but the adapters must fit the Workers runtime.
- Cloudflare's daily limits shape the real-time architecture: the server must not run a 1-second price loop on alarms, and live prices to clients need WebSockets.

## Open questions for later tickets

- Real-time architecture: how the server observes the End price within the Durable Object request and row-write budget (alarm per Guess with re-checks, versus a Cron Trigger at 1-minute granularity, versus clients showing live prices fetched straight from the price source while the server fetches only at placement and resolution).
- Price source: whether the chosen BTC/USD API accepts requests from Cloudflare Worker egress IPs and stays within its own free rate limits.
- Abuse protection: on Cloudflare a flood fails the game closed until 00:00 UTC; rate limiting must protect the daily budget.
- Oracle: whether a Pay As You Go upgrade, kept within Always Free, exempts instances from idle reclamation, if the VM path is chosen.
- Supabase: what counts as inactivity for pausing, if Supabase is reconsidered for its data store.
