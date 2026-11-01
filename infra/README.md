# Infrastructure

Terraform describes the Render service of Hoof & Claw (ADR 0003, ADR 0013).
You apply it by hand; CI never does.

## Apply

Set `RENDER_API_KEY` and `RENDER_OWNER_ID` in the environment, then run:

```sh
terraform -chdir=infra init
terraform -chdir=infra apply
```

`apply` prints the service's `onrender.com` host as `url`.
At All-Inkl, point a `CNAME` for `hoof-and-claw` to that host, once and by hand.

## State

The state lives in `infra/terraform.tfstate` on the machine that applies, and Git ignores it.
There is no remote backend and no backup.

## Recover a lost state

If the state is lost, `apply` would try to create a second service.
Import the existing one instead.
Take the service ID (`srv-…`) from the Render dashboard, then run:

```sh
terraform -chdir=infra init
terraform -chdir=infra import render_web_service.hoof_and_claw <service-id>
terraform -chdir=infra plan
```

The plan should show no changes.
