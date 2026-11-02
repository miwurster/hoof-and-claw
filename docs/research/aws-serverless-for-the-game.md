# Is serverless AWS a poor fit for Hoof & Claw?

Research to re-check the AWS serverless option that ADR 0003 did not choose.
All sources were read on 2026-10-31 unless a line says otherwise.
Terms follow `GLOSSARY.md`.

## Question

Did ADR 0003 dismiss serverless AWS too quickly?
ADR 0003 gave two reasons.
First, "nothing holds the price feed, so browsers poll a cached price and the server reads trades over REST".
Second, "EventBridge Scheduler fires with 60-second precision, so a guess nobody watches resolves up to about two minutes after it falls due".

The rules a design must keep, from the brief and ADRs 0002, 0003, 0007 and 0010:

- A Guess resolves on the first price the server receives at least 60 s after the Guess that differs from the Start price, with no browser open.
- Players always see the latest price, today pushed over SSE at most once a second, and their Score.
- No visible cold start.
- The demo may spend at most 10 EUR a month.

## Answer in short

ADR 0003 was right about the timer and wrong about the feed.

- **The feed.** A Lambda function can hold Bitstamp's WebSocket.
  An invocation runs for at most 15 minutes, so an EventBridge Scheduler rule that starts a new invocation every 13 minutes keeps one connection open at all times, with the old and new invocation overlapping for about two minutes.
  At 128 MB around the clock this uses about 374,000 GB-s a month, inside Lambda's always-free 400,000 GB-s.
  That function can resolve due Guesses on every trade, just as the Render process does, so no timer per Guess is needed and a Guess resolves on the first changed trade after it falls due.
- **The timer.** EventBridge Scheduler does have 60-second precision.
  SQS message timers and Step Functions Wait states take whole seconds, and DynamoDB TTL deletes "within a few days".
  With a feed function, none of them is needed.
- **Push to browsers.** Every managed AWS push service bills each message delivered to each client, and AppSync Events, IoT Core and API Gateway WebSockets all cost about 1 USD per million.
  CloudFront cannot fan one SSE stream out to many viewers, so a cheap price channel means each browser polling a short-TTL cached price through CloudFront, which bills per request.
- **Cost at demo scale.** About 0 to 2 USD a month on always-free tiers, and still under 10 USD if Lambda's free tier were used up.
  Serverless fits the 10 EUR cap and meets every rule of the brief, so at demo scale it is viable.
- **Cost at scale.** One price a second to every Player costs about 2,600 to 3,100 USD a month at 1,000 concurrent Players and ten times that at 10,000, against roughly 60 and 600 USD of bandwidth for an always-on SSE server (ADR 0003).
  Serverless gets more expensive than the current design as Players grow, the opposite of ADR 0003's scale path, which names AWS serverless as its first candidate.

## Findings

"Verified" lines come from the cited source.
Lines marked _(inference)_ are reasoning on top of the sources.
Prices are US East (N. Virginia) unless a line says otherwise, because most AWS pricing pages show only that Region in text.

### 1. Timing: what resolves a Guess within a few seconds of falling due

| Mechanism | Precision | Limits | Cost per Guess | Fit |
| --- | --- | --- | --- | --- |
| EventBridge Scheduler one-time schedule | 60 s | None relevant | 1 invocation, 14 M a month free | Late by up to 59 s, or about 2 min if rounded up to be safe |
| SQS message timer | Whole seconds; no precision stated | 0 to 900 s, standard queues only | About 3 requests, 1 M a month free | Plausible, not documented |
| Step Functions Standard Wait | Whole seconds; timestamps truncated to seconds | Up to 1 year | 3 to 4 state transitions at 0.025 USD per 1,000 | Plausible, not documented |
| Step Functions Express Wait | Whole seconds | Whole workflow at most 5 minutes | Billed for duration, waiting included _(inference)_ | Plausible, not documented |
| DynamoDB TTL | "within a few days" | n/a | Free | No |
| Lambda durable functions wait | Not stated | Up to 1 year | About 8 USD per million operations | No Rust SDK |
| A Lambda that holds the feed and resolves on every trade | Next trade after the Guess falls due | 15 min per invocation | Shared, not per Guess | Yes, same as today |

Verified:

- "All schedule types on EventBridge Scheduler invoke their targets with 60 second precision. This means that if you set your schedule to run at `1:00`, it will invoke the target API between `1:00:00` and `1:00:59`" ([Scheduler schedule types](https://docs.aws.amazon.com/scheduler/latest/UserGuide/schedule-types.html)).
- A one-time schedule uses `at(yyyy-mm-ddThh:mm:ss)`, and a completed one still counts against the account quota until deleted ([Scheduler schedule types](https://docs.aws.amazon.com/scheduler/latest/UserGuide/schedule-types.html)).
- Scheduler gives 14,000,000 invocations a month for free and charges 1.00 USD per million after that ([EventBridge pricing](https://aws.amazon.com/eventbridge/pricing/)).
- An SQS message timer hides a message for 0 s to 15 minutes; "FIFO queues don't support timers on individual messages" ([SQS message timers](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-message-timers.html), [SQS delay queues](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-delay-queues.html)).
  Neither page states how precisely a message becomes visible.
- SQS: "All customers can make 1 million Amazon SQS requests for free each month", and each 64 KB chunk of a payload is billed as one request ([SQS pricing](https://aws.amazon.com/sqs/pricing/)).
- A Step Functions `Wait` takes `Seconds` as an integer from 0 to 99,999,999 or a `Timestamp`, and "if you specify the wait time as a timestamp, Step Functions considers the time value up to seconds and truncates milliseconds".
  The maximum wait is one year for Standard and five minutes for Express workflows ([Wait state](https://docs.aws.amazon.com/step-functions/latest/dg/state-wait.html)).
- Standard workflows cost 0.000025 USD per state transition, with 4,000 free a month; Express costs 1.00 USD per million requests plus 0.00001667 USD per GB-second, and its duration runs "from the time your workflow begins executing until it completes" ([Step Functions pricing](https://aws.amazon.com/step-functions/pricing/)).
- "DynamoDB automatically deletes expired items within a few days of their expiration time" ([DynamoDB TTL](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/TTL.html)).
- Lambda durable functions can wait up to a year without compute charges, but the SDK is available for "JavaScript, TypeScript, Python, Java, Go, and C# (.NET)", not Rust ([durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-functions.html), [durable execution SDK](https://docs.aws.amazon.com/lambda/latest/dg/durable-execution-sdk.html)).
  Each wait is one operation, and the pricing example uses 8.00 USD per million operations ([Lambda pricing](https://aws.amazon.com/lambda/pricing/)).

Inferences:

- ADR 0003's "up to about two minutes" holds only if the schedule is rounded up to the next whole minute.
  The docs do not say whether an `at()` time with seconds fires up to 59 s after that time or anywhere in its minute, which could be before the Guess is due.
  The safe choice is to round up, and then a Guess due at `hh:mm:01` fires as late as `hh:(mm+1):59`, about two minutes later.
- A timer only says when to look.
  Without a feed, the resolver must still find the first changed price after the due time, which brings back the REST question in section 2.
- With a function that holds the feed (section 2), resolution happens on the trade itself, as in ADR 0007, and no timer per Guess is needed.

### 2. Price feed: holding Bitstamp's WebSocket without a server

Verified:

- Lambda code "can run for up to 15 minutes in a single invocation", memory ranges from 128 MB to 10,240 MB, and "Lambda is designed for short-lived compute tasks that do not retain or rely upon state between invocations" ([Lambda quotas](https://docs.aws.amazon.com/lambda/latest/dg/gettingstarted-limits.html)).
- Lambda's free tier is 1 million requests and 400,000 GB-seconds a month, listed as always free; x86 duration costs 0.0000166667 USD per GB-second and requests 0.20 USD per million ([Lambda pricing](https://aws.amazon.com/lambda/pricing/), [AWS free serverless](https://aws.amazon.com/free/serverless/)).
- New AWS accounts get up to 200 USD in credits on a Free or Paid plan, and "30+ AWS services are always free within monthly usage limits on both the Free and Paid plans" ([AWS Free Tier](https://aws.amazon.com/free/)).
- Lambda's recursive loop detection stops a chain after about 16 invocations, including "loops comprised only of Lambda Event Functions, which might invoke each other synchronously or asynchronously"; a function can opt out with `put-function-recursion-config --recursive-loop Allow` ([recursive loop detection](https://docs.aws.amazon.com/lambda/latest/dg/invocation-recursion.html)).
- Rust on Lambda runs on an OS-only runtime with the Rust runtime client ([Rust on Lambda](https://docs.aws.amazon.com/lambda/latest/dg/lambda-rust.html)), and AWS announced Rust support as generally available on 2025-11-14 ([What's New](https://aws.amazon.com/about-aws/whats-new/2025/11/aws-lambda-rust/)).
- Fargate's smallest task is 0.25 vCPU with 0.5 GB; Linux/Arm in US East costs 0.0000089944 USD per vCPU-second and 0.0000009889 USD per GB-second, and Fargate Spot runs "at up to a 70% discount" ([Fargate pricing](https://aws.amazon.com/fargate/pricing/)).
- A Fargate Spot task gets "a two-minute warning", and "a service with only one task is interrupted until capacity is available" ([Fargate capacity providers](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/fargate-capacity-providers.html)).
- A public IPv4 address costs 0.005 USD an hour, in use or idle ([VPC pricing](https://aws.amazon.com/vpc/pricing/)).
- On 2026-10-31, `ws.bitstamp.net` resolved by `CNAME` to `websocket-1667196836.eu-central-1.elb.amazonaws.com` with no `AAAA` record (own `dig` lookup).
  Bitstamp's feed runs behind an AWS load balancer in Frankfurt and is reachable over IPv4 only.
- Bitstamp REST: "As standard, all clients can make 400 requests per second. There is a default limit threshold of 10,000 requests per 10 minutes in place." ([Bitstamp HTTP API](https://www.bitstamp.net/api/), section "Request limits").
- `GET /api/v2/transactions/{market_symbol}/` takes `time` of `minute`, `hour` (default) or `day`, and each trade carries `date` in whole Unix seconds, `tid`, `price` as a string, `amount` and `type` ([Bitstamp HTTP API](https://www.bitstamp.net/api/); a live call on 2026-10-31 returned the same fields).
- The ticker `GET /api/v2/ticker/btcusd/` returns `last` and a `timestamp` in whole seconds (live call on 2026-10-31; ADR 0007 already records this).

Inferences:

- **A feed function.** A Rust function with 128 MB, started by an EventBridge Scheduler rule `rate(13 minutes)` and running until just before its 900 s timeout, always has one invocation connected.
  Scheduler may start each run up to 59 s late, so 13 minutes leaves at least a minute of overlap.
  It runs 24 × 3,600 × 30 × 15/13 ≈ 2.99 M seconds a month, about 374,000 GB-s, inside the 400,000 GB-s free tier but leaving little for the other functions.
  A 14-minute rate with 128 MB gives about 347,000 GB-s and still overlaps by up to a minute.
  Without the free tier it would cost about 6 USD a month at the x86 rate.
  Starting each run from the scheduler, not from the previous run, keeps recursive loop detection out of the way.
- **Using Lambda against its design.** AWS describes Lambda as being for short-lived work, and nothing in the docs promises that an invocation keeps an outbound WebSocket for its full 15 minutes.
  The overlap covers a run that dies early only if the next one has already started, so a crash in mid-run can leave the feed down for up to 13 minutes.
  The design needs an alarm on missing heartbeats, and possibly a second, staggered rule.
- **Resolving in the feed function.** On each trade it writes the latest price to one DynamoDB item and queries a sparse index of Open guesses by due time for Guesses with `dueAt <= receivedAt`, then resolves them with conditional writes.
  Two overlapping invocations may both see the same trade; the conditional write lets the first one win, and both pick the same trade as End price because both apply the same rule.
  This keeps ADR 0002's rule, but the "server" is now whichever feed invocation received the trade.
- **Two clocks.** The API function stamps `placedAt` and the feed function stamps `receivedAt`, on different hosts.
  ADR 0002 chose one clock to avoid skew at the 60 s boundary, so this design would need ADR 0002 to accept clock skew between Lambda hosts, which the docs read here do not quantify.
- **Fargate Spot as the feed holder** needs no load balancer, because nothing connects to it.
  Arm at 0.25 vCPU and 0.5 GB costs about 7.2 USD a month on demand, about 2.2 USD on Spot at the full 70 % discount, plus 3.65 USD for the public IPv4 address that Bitstamp's IPv4-only endpoint requires, so about 6 to 11 USD in total.
  A one-task Spot service is down until capacity returns after an interruption, so the Lambda feed is both cheaper and no less available.
- **REST instead of a feed.** A resolver that fires at or after the due time can read `transactions?time=hour` and take the first trade with `date >= dueAt` whose price differs from the Start price.
  That changes the End price rule from server receipt time to Bitstamp's trade time, which ADR 0002 rejected, and Bitstamp's `date` has only whole seconds, so trades in the due second are ambiguous.
  One shared poll a second uses 600 of the 10,000 requests per 10 minutes; a REST call per Guess runs out at about 16 Guesses a second, roughly 1,000 Players.
  The feed function makes this unnecessary.

### 3. Push to browsers

| Channel | Limits | Price | Free tier |
| --- | --- | --- | --- |
| API Gateway WebSocket API | 2 h per connection, 10 min idle, 32 KB frames, 500 new connections/s | 1.00 USD per million messages, 0.25 USD per million connection minutes, metered per 32 KB | 1 M messages and 750,000 connection minutes a month for 12 months |
| AWS IoT Core (MQTT over WebSocket) | 24 h per WebSocket connection, 100 publishes/s per connection, 20,000 outbound publishes/s per account | 1 USD per million messages, 0.08 USD per million connection minutes, metered per 5 KB | 500,000 messages and 2.25 M connection minutes for 12 months |
| AppSync Events | 24 h per connection, 1,000,000 outbound messages/s per API | 1.00 USD per million operations, 0.08 USD per million connection minutes, metered per 5 KB | 250,000 updates and 600,000 connection minutes for 12 months |
| Lambda response streaming (SSE from a function URL) | 15 min per invocation, 2 MB/s after the first 6 MB | Full duration of every open stream, 0.008 USD per GB streamed past 100 GB | Shares Lambda's free tier |
| CloudFront in front of a cached price | Origin response timeout between packets; no fan-out of a live stream _(inference)_ | 0.0120 USD per 10,000 HTTPS requests in Europe, 0.0100 in the US, 0.085 USD per GB after 1 TB | 1 TB and 10 M requests a month, always free |

Verified:

- API Gateway WebSocket APIs: "Connection duration for WebSocket API: 2 hours", "Idle Connection Timeout: 10 minutes", 32 KB frames, 500 new connections a second per account ([API Gateway WebSocket quotas](https://docs.aws.amazon.com/apigateway/latest/developerguide/apigateway-execution-service-websocket-limits-table.html)).
  The pricing example uses 1.00 USD per million messages and 0.25 USD per million connection minutes, "messages are metered in 32KB increments", and the free tier is "one million messages and 750,000 connection minutes for WebSocket APIs per month for up to 12 months" ([API Gateway pricing](https://aws.amazon.com/api-gateway/pricing/)).
- IoT Core: "The WebSocket connection lifetime is 24 hours", the keep-alive is at most 1,200 s, and each connection may make 100 publish requests a second ([IoT Core quotas](https://docs.aws.amazon.com/general/latest/gr/iot-core.html)).
  Messaging costs 1 USD per million for the first billion messages, connectivity 0.08 USD per million minutes, messages are metered in 5 KB increments, delivered messages are billed separately, and the free tier lasts 12 months ([IoT Core pricing](https://aws.amazon.com/iot-core/pricing/)).
- AppSync Events: a connection "remains open as long as the client remains connected, up to a maximum of 24 hours"; the server sends a keep-alive every 60 s and the client times out after 5 minutes ([Event API WebSocket protocol](https://docs.aws.amazon.com/appsync/latest/eventapi/event-api-websocket-protocol.html)).
  Outbound messages are capped at 1,000,000 a second per API, and one metered event is 5 kB ([AppSync quotas](https://docs.aws.amazon.com/general/latest/gr/appsync.html)).
  The price is 1.00 USD per million Event API operations, counting inbound publishes, outbound broadcasts and WebSocket operations, plus 0.08 USD per million connection minutes; the pricing example bills 10 million outbound messages for 10 million deliveries ([AppSync pricing](https://aws.amazon.com/appsync/pricing/)).
- Lambda response streaming works through function URLs, `InvokeWithResponseStream` and API Gateway; for languages other than Node.js it needs a custom runtime integration; "streamed responses are not interrupted or stopped when the invoking client connection is broken. Customers are billed for the full function duration" ([response streaming](https://docs.aws.amazon.com/lambda/latest/dg/configuration-response-streaming.html)).
  Streaming costs 0.008 USD per GB with 100 GB a month free ([Lambda pricing](https://aws.amazon.com/lambda/pricing/)).
- CloudFront request collapsing: when "additional requests for the same object (with the same cache key) arrive at the edge location before CloudFront receives the response to the first request", CloudFront pauses them and "sends the response from the original request to all the requests that it received while it was paused" ([CloudFront custom origins](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/RequestAndResponseBehaviorCustomOrigin.html)).
- For a chunked response, CloudFront "returns the object to the client as the object is received at the edge location, and caches the object in chunked format for subsequent requests", and "if the chunked response is not complete, CloudFront does not cache the object" ([CloudFront custom origins](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/RequestAndResponseBehaviorCustomOrigin.html)).
- The origin response timeout also applies to "the amount of time … that CloudFront waits after receiving a packet of a response from the origin and before receiving the next packet" ([CloudFront custom origins](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/RequestAndResponseBehaviorCustomOrigin.html)).
- CloudFront's always-free tier is 1 TB out and 10,000,000 HTTP or HTTPS requests a month; past that, HTTPS requests cost 0.0120 USD per 10,000 in Europe and 0.0100 in the US ([CloudFront pay-as-you-go](https://aws.amazon.com/cloudfront/pricing/pay-as-you-go/)).
- CloudFront's flat-rate plans are Free (0 USD, 1 M requests, 100 GB), Pro (15 USD, 10 M requests, 50 TB) and Business (200 USD, 125 M requests, 50 TB), with "No overage charges" ([CloudFront pricing](https://aws.amazon.com/cloudfront/pricing/)).
  The page read does not say what happens to traffic beyond a plan's allowance.

Inferences:

- CloudFront cannot fan one SSE stream out to many viewers.
  An SSE response never completes, so it is never cached, and collapsing covers only requests that arrive before the first response; no page describes sharing a response that is still streaming.
  Every SSE viewer would hold its own stream to the origin, so CloudFront saves nothing.
- The cheap serverless price channel is polling.
  The feed function writes the latest price to DynamoDB, a function URL serves it with `Cache-Control: max-age=1`, and CloudFront collapses the requests, so the origin sees about one request a second per edge location.
  A Player sees a price at most about 2 s old, up to 1 s in cache plus up to 1 s between polls.
  ADR 0010's SSE stream lags by at most 1 s.
- The managed push services all bill each message to each client, so one price a second to N Players costs about N × 2.6 USD a month around the clock, whichever service carries it.
  Sending prices only when they change lowers this.
  On 2026-10-31 Render Frankfurt saw 1,159 trades in an hour (ADR 0007), at most 0.32 changes a second, so about a third of the cost.
- SSE from a Lambda function URL is the most expensive channel, because every viewer keeps a function running for the whole time it watches.

### 4. Cold starts

Verified:

- SnapStart supports Java 11+, Python 3.12+ and .NET 8+; "OS-only runtimes are not supported" for `.zip` functions, but container images built on `provided.al2023` can use SnapStart by implementing the restore hooks or by setting `LABEL com.amazonaws.lambda.feature.snapstart="Allow"` ([SnapStart](https://docs.aws.amazon.com/lambda/latest/dg/snapstart.html), [SnapStart for container images](https://docs.aws.amazon.com/lambda/latest/dg/snapstart-runtime-hooks-custom.html)).
- Provisioned concurrency costs 0.0000041667 USD per GB-s while provisioned plus 0.0000097222 USD per GB-s of duration, and the free tier does not apply to functions that use it ([Lambda pricing](https://aws.amazon.com/lambda/pricing/)).
- Provisioned concurrency "keeps Event Functions initialized and ready to respond in double-digit milliseconds" ([SnapStart](https://docs.aws.amazon.com/lambda/latest/dg/snapstart.html)).

Secondary source, not an AWS page:

- A Classmethod benchmark of 2026-09-04 measured a minimal Rust container function at 512 MB in Tokyo at 19 ms server-side cold start (init plus handler) and a client-side p50 of 497 ms from Japan; SnapStart made it slower, at 377 ms server-side and 563 ms client-side ([Classmethod](https://dev.classmethod.jp/en/articles/lambda-snapstart-container-benchmark/)).
  Only a function with seconds of initialization gained from SnapStart.
  AWS publishes no Rust cold-start figures in the pages read.

Inferences:

- A Rust function that does little at start-up has a cold start of tens of milliseconds on the server, which a Player will not notice; SnapStart would make it worse.
- The feed function never has a visible cold start, because it is always running and Players never call it.
- Keeping one API function warm with provisioned concurrency at 128 MB costs about 0.125 GB × 2.59 M s × 0.0000041667 USD ≈ 1.35 USD a month plus duration, which still fits the cap if wanted.
- The SPA is static files from S3 through CloudFront, with no function on the path.

### 5. Cost

Assumptions, as in ADR 0003:

- Demo: 20 concurrent Players, 4 hours a day, 30 days a month, each placing one Guess a minute and polling the price once a second.
  That is 8.64 M price polls and 144,000 Guesses a month.
- Scale: 1,000 and 10,000 concurrent Players around the clock, each placing about one Guess a minute and receiving one price a second.
- The feed function runs around the clock at every scale.

Demo, best serverless design (section 6), one month:

| Item | Usage | Cost |
| --- | --- | --- |
| Feed function, 128 MB | About 374,000 GB-s, 3,300 invocations | 0 USD (always free) |
| API functions, 128 MB | About 2.5 M requests (Guesses and Player polls), about 6,000 GB-s | About 0.30 USD for requests past 1 M |
| EventBridge Scheduler | About 3,300 invocations | 0 USD |
| DynamoDB | About 0.3 writes a second for prices, 0.14 for Guesses, one query per trade | 0 USD with provisioned capacity inside 25 WCU and 25 RCU; about 1 USD on demand |
| CloudFront | About 11 M requests, a few GB | About 1.20 USD for requests past 10 M |
| S3 for the SPA | A few MB | Cents |
| Total | | **About 0 to 2 USD a month** |

Inferences on the demo:

- Without Lambda's free tier, the feed function costs about 6 USD and the rest stays under 3 USD, so the demo still stays under the 10 EUR cap.
  The CloudFront and DynamoDB allowances are always free on every account; without them, CloudFront alone would cost about 13 USD for 11 M requests.
- Replacing Player polls with AppSync Events for Player events adds about 0.3 USD a month (2 messages per Guess); pushing prices through AppSync Events too would add about 2.8 USD at 0.32 changes a second, or 8.6 USD at one a second, which breaks the cap.
- CloudWatch Logs is not counted; a chatty feed function could cost more than everything else, so logs need to stay small.

Scale, one month around the clock (2.59 M seconds), price channel only:

| Concurrent Players | Price deliveries at 1/s | CloudFront polling (Europe) | AppSync Events or IoT Core or API Gateway WebSocket | Always-on SSE, bandwidth only (ADR 0003) |
| --- | --- | --- | --- | --- |
| 1,000 | 2.59 billion | About 3,100 USD | About 2,600 USD | About 58 USD on Render |
| 10,000 | 25.9 billion | About 31,000 USD | About 26,000 USD | About 600 USD on Render |

Guesses at scale, serverless:

| Concurrent Players | Guesses a month | Lambda | DynamoDB on demand (about 2.5 writes per Guess) | Player events via AppSync (2 per Guess) |
| --- | --- | --- | --- | --- |
| 1,000 | 43 M | About 10 USD | About 70 USD | About 90 USD |
| 10,000 | 432 M | About 100 USD | About 700 USD | About 900 USD |

Inferences on scale:

- The price channel dominates.
  Every per-message service costs about 1 USD per million deliveries, while an SSE server pays only for bytes, about 150 bytes per price.
  Pushing only changes (about 0.32 a second) brings the price channel to about 830 USD at 1,000 Players, still more than ten times the always-on bandwidth cost.
- Guess handling stays reasonable on serverless at both scales; it is the fan-out of prices that does not.
- The cheapest AWS design at scale is therefore a hybrid of serverless for Guesses and resolution and an always-on SSE tier for prices (for example on Fargate or EC2 with its own egress), which ADR 0010's `Fanout` port already allows.

### 6. Verdict and the best serverless design

The best serverless design for the demo, in `eu-central-1` next to Bitstamp's load balancer:

- **Static SPA** in S3 behind CloudFront, on `hoof-and-claw.miwurster.com` with an ACM certificate in `us-east-1` and a `CNAME` at All-Inkl.
- **Feed function** in Rust, 128 MB, started every 13 minutes by EventBridge Scheduler, running up to 900 s.
  It holds `live_trades_btcusd`, seeds from the REST ticker on connect, sends heartbeats as ADR 0007 requires, and on each trade:
  1. writes the latest price, its `receivedAt` and health to one DynamoDB item;
  2. queries Open guesses with `dueAt <= receivedAt` and resolves those whose Start price differs, with conditional writes that adjust the Score.
- **API function** in Rust behind CloudFront, as a function URL with origin access control, to place a Guess and read a Player's snapshot.
  Placing a Guess reads the latest-price item; if its health is stale, the price counts as Price delayed and the Guess is refused.
- **Price channel.** Browsers poll `GET /price` through CloudFront with `max-age=1` once a second.
- **Player events.** The client polls its snapshot every couple of seconds while a Guess is due, or subscribes to an AppSync Events channel per Player.
- **Store.** DynamoDB replaces SQLite behind the existing store port.

What it gives up against the current design:

- **One process and one clock.** `placedAt` and `receivedAt` come from different hosts, which ADR 0002 avoided.
- **Push within 1 s.** Prices arrive by polling, up to about 2 s old, unless a per-message push service is paid for.
- **The in-memory index.** Every trade costs a DynamoDB query.
- **Simplicity.** One binary and one disk become a scheduler, two functions, a table with an index, a bucket, a distribution and IAM policies, and the acceptance suite needs a deployed stack or emulators instead of one local process.
- **A known failure mode.** Lambda is not meant to hold connections; the design depends on the overlap of scheduled runs and needs monitoring for gaps.
- **Scale economics.** Past a few hundred Players the price channel costs more than an always-on server, so the scale path needs an always-on fan-out tier anyway.

What it gains:

- About 0 to 2 USD a month instead of 7.25 USD, and "AWS preferred" met.
- No deploy downtime for resolution, because a new feed function takes over at the next scheduled run and API functions deploy without dropping anyone.
- No disk to lose, because DynamoDB keeps the Scores, which removes ADR 0003's "no database backup" risk.

ADR 0003 dismissed serverless AWS partly on a wrong premise.
"Nothing holds the price feed" is not true, because a scheduled Lambda can hold it inside the free tier and resolve Guesses on every trade, so the 60-second precision of EventBridge Scheduler does not matter.
At demo scale serverless AWS meets the brief's rules, fits the 10 EUR cap with a wide margin, and costs less than Render.
It pays for that with polling instead of push, two clocks, and more moving parts.
The ADR's scale path is the part that should change most, because serverless AWS is the cheapest option for the demo and the most expensive for delivering prices at 1,000 Players and above.

## Not investigated

- Lambda Web Functions, Lambda MicroVMs (up to 8 hours per execution) and Lambda Managed Instances, which appeared in the Lambda quotas and pricing pages; any of them might hold the feed or serve SSE more directly than event functions.
- Exact Frankfurt prices for Lambda, DynamoDB, IoT Core and AppSync, which the pricing pages did not show in text.
- How precisely SQS timers and Step Functions waits fire in practice; no page states it, and measuring needs an AWS account.
- Bitstamp's live WebSocket docs, which block automated reads (ADR 0007).
- Bitstamp's "Commercial Use of Bitstamp's Exchange Data" section, which asks companies using the data for commercial purposes to sign a licence; this applies to every hosting option, not just AWS.
