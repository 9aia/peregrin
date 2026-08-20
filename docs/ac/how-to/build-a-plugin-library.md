# Build a plugin library

A plugin library is just a directory of reusable YAML fragments. There is
no package manager, no registry, and no loading ceremony — `include:`
installs a fragment into a workflow.

## CI checks

```yaml
# plugins/ci/lint.yaml
id: lint
driver: shell
on: [start]
emits: [lint.passed, lint.failed]
params:
  cmd: "ruff check ."
```

```yaml
# plugins/ci/test.yaml
id: test
driver: shell
on: [start]
emits: [test.passed, test.failed]
params:
  cmd: "pytest -q"
```

```yaml
# plugins/ci/coverage.yaml
id: coverage
driver: shell
on: [test.passed]
emits: [coverage.passed, coverage.failed]
params:
  cmd: "pytest --cov --cov-fail-under=80"
```

Use them:

```yaml
name: my-ci
nodes:
  - include: plugins/ci/lint.yaml
  - include: plugins/ci/test.yaml
  - include: plugins/ci/coverage.yaml

  - id: summary
    driver: shell
    on: [lint.passed, test.passed, coverage.passed]
    mode: all
    emits: [ci.passed]
    params:
      cmd: "echo 'all checks passed'"

  - id: alert
    driver: shell
    on: [lint.failed, test.failed, coverage.failed]
    mode: any
    emits: [ci.failed]
    params:
      cmd: "echo 'CI failed'"
```

## Notifications

```yaml
# plugins/notify/slack.yaml
id: notify-slack
driver: shell
on: [notify.requested]
emits: [notify.done]
params:
  cmd: 'curl -X POST -H ''Content-type: application/json'' --data ''{"text":"${event.payload.message}"}'' ${secret.SLACK_WEBHOOK_URL}'
```

```yaml
# plugins/notify/email.yaml
id: notify-email
driver: shell
on: [notify.requested]
emits: [notify.done]
params:
  cmd: "echo '${event.payload.message}' | mail -s 'Notification' ${event.payload.to}"
```

```yaml
# plugins/notify/pagerduty.yaml
id: notify-pagerduty
driver: shell
on: [alert.critical]
emits: [notify.done]
params:
  cmd: "pagerduty-cli trigger --severity critical --description '${event.payload.message}'"
```

Use them:

```yaml
name: deploy-pipeline
nodes:
  - include: plugins/notify/slack.yaml
  - include: plugins/notify/pagerduty.yaml

  - id: build
    driver: shell
    on: [start]
    emits: [build.done, build.failed]
    params:
      cmd: "make build"

  - id: deploy
    driver: shell
    on: [build.done]
    emits: [deploy.done, deploy.failed]
    params:
      cmd: "make deploy"

  - id: notify-success
    driver: shell
    on: [deploy.done]
    emits: [notify.requested]
    params:
      cmd: "echo 'deploy succeeded'"

  - id: alert-failure
    driver: shell
    on: [deploy.failed]
    emits: [alert.critical]
    params:
      cmd: "echo 'deploy failed'"
```

## Shared config

```yaml
# snippets/prod-env.yaml
env:
  PYTHONPATH: /app
  LOG_LEVEL: INFO
  REGION: us-east-1
timeout: 300
```

```yaml
# snippets/staging-env.yaml
env:
  PYTHONPATH: /app
  LOG_LEVEL: DEBUG
  REGION: eu-west-1
timeout: 60
```

Use them:

```yaml
nodes:
  - id: deploy
    driver: shell
    on: [start]
    emits: [deploy.done]
    params:
      include: snippets/prod-env.yaml
      cmd: "deploy.sh"
      timeout: 60
```

## See also

- [Plugins](../explanation/plugins.md) — why YAML files are plugins
- [Compose from fragments](compose-from-fragments.md) — `include:` and `${include()}` details
