# edchecker
edchecker checks consistency of a trace with SC+Event semantics. A proof-of-concept, that can be incorporated in e.g. DPOR-frameworks.

## Build
```sh
cargo build --release
```

## Basics
Given a trace with `rf`, `co` and `pb`-edges, we search for an `eo`; an execution order between messages in a given handler.
There are two algorithms included. The first is a naive version that exhaustively tests each `eo`, and the second uses Z3. They are named `edchecker` and `z3checker`, respectively. The tools can be run with the command

```sh
cargo run --release --bin <bin> <adt> <heuristics> <trace> 
```

where `adt` is one of `multiset`, `queue`, `stack`, `priority-queue`, or `register`, and `heuristics` is one of `no`, `simple` or `full`.

The heuristics only have an effect on `edchecker`.
The implemented `adt`s are `stack`, `queue`, `priority-queue` and `multiset`.
