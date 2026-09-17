# Architecture (Reference)

The shape of this project: its components, where they live, and how a request or job flows
through them. This page describes *what is*; rationale belongs in [`explanation/`](../explanation/)
and hard-to-reverse decisions in [`adr/`](../adr/README.md). Coding rules live in the area docs in
[`conventions/`](../conventions/). The `architecture-explainer` subagent grounds its answers here.

<!-- TODO(adapt): one or two sentences on what the system does and its overall style (single service, monorepo of apps + libraries, CLI, library, ...). -->

## Topology

<!-- TODO(adapt): a small ASCII or Mermaid diagram of the runtime components and the protocols between them (HTTP, queues, RPC, DB). Omit for a single-process project. -->

```
<client> ──▶ <entry point> ──▶ <core logic> ──▶ <data store / external system>
```

## Components

| Component | Path | Owns | Holds state? |
| :-------- | :--- | :--- | :----------- |
| <!-- TODO(adapt): name --> | <!-- TODO(adapt): e.g. `src/api/` --> | <!-- TODO(adapt): responsibility, in one line --> | <!-- TODO(adapt): no / what state --> |

## Directory layout

<!-- TODO(adapt): the top-level tree with a one-line purpose per directory. Stop at the depth where a reader can find the right place for a new file. -->

```
<root>/
  <dir>/        <purpose>
```

## Request and data flow

<!-- TODO(adapt): one numbered path per distinct entry point (HTTP request, background job, message consumer, CLI command). Name each layer it crosses and where validation, authorization, and persistence happen. -->

### Path A: <!-- TODO(adapt): e.g. an HTTP API request -->

1. <!-- TODO(adapt): entry -->
2. <!-- TODO(adapt): validation / auth -->
3. <!-- TODO(adapt): business logic -->
4. <!-- TODO(adapt): persistence / side effects -->
5. <!-- TODO(adapt): response -->

## External dependencies

| Dependency | Used for | Accessed from | Failure behaviour |
| :--------- | :------- | :------------ | :---------------- |
| <!-- TODO(adapt): database, queue, third-party API, identity provider, ... --> | | | <!-- TODO(adapt): fail fast / retry / degrade --> |

## Deployment topology

<!-- TODO(adapt): what is deployed where (processes, containers, functions), how each scales (replicas, affinity, partitions), and environments. Write "not deployed" for a library. -->
