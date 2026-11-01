# The Render service of Hoof & Claw (ADR 0003, ADR 0013), applied by hand.
# State lives only on the machine that applies it, outside Git; infra/README.md says how to apply and how to recover it.
# The provider reads RENDER_API_KEY and RENDER_OWNER_ID from the environment.
# The CNAME of the custom domain at All-Inkl is set by hand.

terraform {
  required_version = "~> 1.16"

  required_providers {
    render = {
      source  = "render-oss/render"
      version = "~> 1.9"
    }
  }
}

provider "render" {}

resource "render_web_service" "hoof_and_claw" {
  name   = "hoof-and-claw"
  plan   = "starter"
  region = "frankfurt"

  # Render builds the Dockerfile on every commit to `release`, which `mise run release` fast-forwards.
  runtime_source = {
    docker = {
      repo_url            = "https://github.com/miwurster/hoof-and-claw"
      branch              = "release"
      dockerfile_path     = "./Dockerfile"
      context             = "."
      auto_deploy_trigger = "commit"
    }
  }

  # Answers once migrations ran, also while the price is delayed, so a Bitstamp outage restarts nothing.
  health_check_path = "/health"

  # max_shutdown_delay_seconds stays unset: a service with a disk cannot change it.
  disk = {
    name       = "data"
    mount_path = "/var/data"
    size_gb    = 1
  }

  env_vars = {
    DATABASE_PATH = { value = "/var/data/hoof-and-claw.db" }
    LOG_FORMAT    = { value = "json" }
  }

  custom_domains = [
    { name = "hoof-and-claw.miwurster.com" },
  ]
}

output "url" {
  description = "The service's onrender.com address, which the custom domain's CNAME points to."
  value       = render_web_service.hoof_and_claw.url
}
