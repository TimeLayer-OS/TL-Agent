# TL-Agent

**An agent cannot authorize its own actions. And cannot rewrite the history of what it did.**

TL-Agent gives an AI agent its permissions as notarial receipts from the [TimeLayer](https://timelayer-os.com) network — receipts it cannot issue itself. Every permitted action is backed by one. The agent's history is tamper-evident and verifiable offline by any third party.

> **NO VALID RECEIPT → NO ACTION**

---

## Why this, not "just add checks in the agent's code"

| Self-built (flags + log in code) | TL-Agent |
|---|---|
| The agent essentially decides itself — it can "authorize" itself to bypass the check | Permission is a **notarial receipt the agent does not issue** (INV-01) |
| Action history is a log at the operator — editable | History is receipts **that cannot be rewritten**, third-party verifiable |
| Trust — in your code and your server | Trust — in a **quorum of independent operators**, offline verification with open-source code |
| "Done" — on the model's word | "Done" — only with a receipt present (INV-06: the model's text is not proof) |

**In one line: a check in the agent's code can be bypassed by the agent; a notarial receipt it cannot issue itself — cannot.**

---

## Three guarantees

- **Action control.** Every action passes a gate: no valid receipt — the action does not run. The topology defines what is permitted at all.
- **Tamper-evident history.** What the agent did is recorded in receipts that neither the agent nor the operator can rewrite. Verifiable offline by a third party.
- **Fail-closed by design.** Any conflict, missing permission, or unknown action → agent stops and waits for a human. Silence is a safe refusal, not a hidden success.

---

## Quick start

### 1. Download `tl-agent`

Grab the latest binary from [Releases](../../releases/latest).

```bash
chmod +x tl-agent
./tl-agent --help
```

### 2. Gate-check an action before running it

```bash
tl-agent check ./example-bundle action_read_files
# ALLOW
#   action  : action_read_files
#   receipt : tlr_20260624_000001
#   next    : action_summarize
```

Exit code `0` = ALLOW, `1` = STOP. Wire this into any agent loop or shell script.

### 3. Audit the whole bundle

```bash
tl-agent audit ./example-bundle
# === tl-agent audit ===
# actions : 4  valid: 4  stop: 0
# result: ALL VALID — bundle is ready for use
```

### 4. In your Rust agent

```rust
let bundle = AgentBundle::load("./agent-bundle")?;

match bundle.check_action("action_read_files") {
    CheckResult::Allow(receipt) => {
        // run — permission is notarially confirmed
    }
    CheckResult::Stop { reason, .. } => {
        // halt — NO_RECEIPT, TOPOLOGY_VIOLATION, SIGNATURE_INVALID, etc.
    }
}
```

---

## Build a bundle

The fastest way is the **[Agent Builder](https://cabinet.timelayer-os.com/agent)** in the TimeLayer cabinet:

1. Define actions and topology visually
2. Each action gets a receipt notarized by the network
3. Download the ready-to-use ZIP

Or create the bundle structure manually per `SPEC.md`.

---

## What's in this repo

| Path | Description |
|------|-------------|
| `timelayer-agent-sdk/` | Rust library + `tl-agent` CLI binary |
| `example-bundle/` | Ready-to-run example bundle with real `.tlsig` receipts |
| `SPEC.md` | Full specification of bundle formats and invariants |

---

## Bundle structure

```
agent-bundle/
  manifest.json          ← bundle ID, owner, receipt count, tl_agent_version
  topology.json          ← allowed transitions between actions
  policies/
    agent_policy.json    ← what the agent may not do
    tool_policy.json     ← which tools are allowed
    stop_policy.json     ← when the agent must halt
  receipts/
    <action_id>/
      envelope.json      ← agent metadata, allowed_next_actions, references proof.tlsig
      proof.tlsig        ← notarial receipt — never modified
  exports/
    segment_01/          ← partial bundle segment for handing to the agent
```

`.tlsig` is issued by the TimeLayer network. TL-Agent only reads and verifies it — never writes or modifies.

---

## CLI reference

```
tl-agent check  <bundle_dir> <action_id>              Gate check. Exit 0=ALLOW, 1=STOP.
tl-agent next   <bundle_dir> <action_id>              List allowed next actions per topology.
tl-agent audit  <bundle_dir>                          Verify every action in the bundle.
tl-agent record <bundle_dir> <action_id> <sha256>     Append to execution_log.jsonl.

Options:
  --verifier <path>    Path to timelayer-verifier binary.
                       Auto-resolved: next to tl-agent → bundle/bin/ → PATH
```

---

## Build from source

Requires Rust 1.70+.

```bash
cd timelayer-agent-sdk
cargo build --release
# → target/release/tl-agent  (807 KB, no runtime deps beyond timelayer-verifier)
```

---

## Security model

| Invariant | Rule |
|-----------|------|
| INV-01 | The agent does not issue valid receipts itself |
| INV-02 | The agent does not modify the user's bundle |
| INV-03 | Every action requires a `permission_receipt` |
| INV-04 | Every transition is validated against `topology.json` |
| INV-05 | Any conflict → STOP |
| INV-06 | The model's text output is not proof |
| INV-09 | `.tlsig` is never modified |

**Fail-closed**: any error (missing receipt, invalid signature, unknown action) → STOP, never silent pass.

---

## Honest framing

- **TimeLayer network**: quorum of independent operators, public keys on GitHub
- **Signatures**: Ed25519; hash commitment is quantum-resilient; post-quantum signing on the roadmap
- **What TL-Agent is**: guardrails + tamper-evident audit for a cooperative agent — not a sandbox that physically locks down malicious code (except in air-gapped mode, and even then only from the agent, not the host)
- **Not memory as recall** — provenance. Receipts do not store content and do not do semantic search
- External network audit: on the roadmap

---

## Learn more

- **Landing page**: [timelayer-os.com/tl-agent](https://timelayer-os.com/tl-agent/)
- **Agent Builder** (cabinet): [cabinet.timelayer-os.com/agent](https://cabinet.timelayer-os.com/agent)
- **Verifier** (open-source): [github.com/TimeLayer-OS/timelayer-verifier](https://github.com/TimeLayer-OS/timelayer-verifier)
- **Docs**: [timelayer-os.com/docs](https://timelayer-os.com/docs/)

---

Part of the [TimeLayer](https://timelayer-os.com) ecosystem.

MIT License
