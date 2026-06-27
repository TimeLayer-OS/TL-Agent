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

## Run from a disk container or a USB flash drive (air-gapped)

A bundle is just a folder, so the **same bundle is portable** — the SDK does not know or
care whether it lives on the local disk or on a mounted drive. That is something logs and
plain audit records cannot give you: a log is tied to the server that wrote it. A notarial
receipt verifies **offline, on any machine, with no connection to us** — so an agent's
permissions can travel on a USB stick and still be tamper-evident and self-verifying.

**Mode 1 — disk container (default).** Keep the bundle as a folder on the host. The agent
loads it at startup, runs every gate check locally, makes no network calls during execution.

```bash
tl-agent check ./agent-bundle action_read_files
```

**Mode 2 — air-gapped USB.** Put the permission bundle on a **read-only** removable drive
and point results at a **separate, append-only** drive. The agent mounts the permissions,
reads, verifies, acts — and can never write back to where its permissions live.

```bash
# permissions mounted read-only; same command, just a different path
tl-agent check /mnt/permits/agent-bundle action_read_files
```

Splitting the media enforces INV-01 (the agent cannot issue receipts) and INV-02 (the agent
cannot modify the bundle) **at the hardware level** — it can't overwrite its own permissions
or inject a fake result.

> **Receipts are immutable (INV-09).** You never edit a receipt on the drive. To grant more
> actions you build a *new* bundle — the new actions get their own notarized receipts and the
> old ones travel alongside, untouched.

**Honest limits.** Hardware write-protect on the USB is real protection; a software
"read-only" flag a privileged host process can bypass. Air-gap protects against the *agent*,
not against a compromised host OS. And verifying existing receipts is fully offline, but
*issuing a new* receipt still needs a moment of connectivity with the quorum. Full write-up:
[Engineering note → Two usage modes](https://timelayer-os.com/docs/tl-agent/#modes).

---

## What's in this repo

| Path | Description |
|------|-------------|
| `timelayer-agent-sdk/` | Rust library + `tl-agent` CLI binary |
| `example-bundle/` | Ready-to-run example bundle with real `cert.tlcert` + `bundle.tlbundle` receipts |
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
      envelope.json      ← agent metadata, allowed_next_actions, references the proof
      cert.tlcert        ← notarial certificate — never modified
      bundle.tlbundle    ← notarial bundle (signatures) — verified together with cert
  exports/
    segment_01/          ← partial bundle segment for handing to the agent
```

`cert.tlcert` + `bundle.tlbundle` are issued by the TimeLayer network. TL-Agent only reads and verifies them — never writes or modifies.

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
| INV-09 | The receipt (`cert.tlcert` + `bundle.tlbundle`) is never modified |

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
