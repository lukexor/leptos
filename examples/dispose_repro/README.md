# dispose_repro

Use-after-dispose panics with `AsyncDerived`, `spawn_local` and a `Memo` read
outside its `Transition`. Pinned to the crates.io releases it was found on
(leptos 0.8.22, reactive_graph 0.2.15, tachys 0.2.19), with a release profile
using `panic = "abort"`.

```sh
cargo leptos build --release
./stress.sh /async-derived-unread 3000 8   # server aborts within ~8 requests
./stress.sh /async-derived-arc 5000 8      # survives
./stress.sh /async-derived 5000 8          # survives
```

`stress.sh <path> <requests> <parallel>` starts the server, loads the path in
parallel and reports whether the server survived. The server log is
`server.log`.

| Route                   | What happens                                                                 |
| ----------------------- | ---------------------------------------------------------------------------- |
| `/async-derived-unread` | An `AsyncDerived` no `Transition` awaits reads a signal after the page is disposed. The server aborts. |
| `/async-derived-arc`    | The same, capturing `ArcResource`/`ArcRwSignal`. Survives.                    |
| `/async-derived`        | The same derived, awaited by a `Transition`. Survives.                       |
| `/spawn`                | In a browser: click "load", then "toggle dialog" within 3 s. The WASM panics. |
| `/memo`                 | Server renders "empty" though the resource has items, and hydration keeps it. |
