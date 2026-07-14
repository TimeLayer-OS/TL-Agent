# TL-Agent

**English** · [Русский](README.ru.md)

[![ci](https://github.com/TimeLayer-OS/TL-Agent/actions/workflows/ci.yml/badge.svg)](https://github.com/TimeLayer-OS/TL-Agent/actions/workflows/ci.yml)
[![receipt-verified](receipt-verified.svg)](https://github.com/TimeLayer-OS/timelayer-verifier/tree/main/audit/2026-07-14)

> **FOR AGENTS.** Gate one action, fail-closed:
> ```bash
> git clone …/TL-Agent && cd TL-Agent
> tl-agent check example-bundle action_read_files   # ALLOW | STOP(reason)
> ```
> Working method: [`AGENTS.md`](AGENTS.md).

**An agent cannot authorize its own actions. And cannot rewrite the history of what it did.**

TL-Agent gives an AI agent its permissions as notarial receipts from the [TimeLayer](https://timelayer-os.com) network — receipts it cannot issue itself. Every permitted action is backed by one. The agent's history is tamper-evident and verifiable offline by any third party.

> **NO VALID RECEIPT → NO ACTION**

## Why you might care

- **You run AI agents** that touch files, money, or customers — and "the model said done"
  is not something you can show a regulator or an angry client.
- **EU AI Act, arts. 12/19** (from 2026-08-02) require tamper-evident records of high-risk
  AI actions. A receipt chain that verifies offline is exactly that, with no log to protect.
- **You need it to work disconnected.** The air-gapped USB mode carries permissions and
  proofs on a stick — for environments where "call the license server" is not an answer.

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

### 1. Download `tl-agent` — pinned and checksum-verified

Never run an unpinned binary. Pick an exact release, download the asset **and** the
`SHA256SUMS.txt` published with it, and verify before you run it:

```bash
VER=v0.2.1
BASE="https://github.com/TimeLayer-OS/TL-Agent/releases/download/$VER"
curl -fsSL "$BASE/tl-agent-linux-x86_64.zip" -o tl-agent.zip
curl -fsSL "$BASE/SHA256SUMS.txt"            -o SHA256SUMS.txt
grep " tl-agent-linux-x86_64.zip$" SHA256SUMS.txt | sha256sum -c -   # must print: OK
unzip -o tl-agent.zip && chmod +x tl-agent
./tl-agent --help
```

Same discipline as [`receipt-driven-examples/run.sh`](https://github.com/TimeLayer-OS/receipt-driven-examples/blob/main/run.sh): a compromised or swapped release must fail the check before execution.

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
tl-agent record <bundle_dir> <action_id> <sha256>     Append to execution_log.jsonl (diagnostics, not proof).
tl-agent intent-digest <envelope.json>                Print the tl-intent/1 commitment to notarize.

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

## What the current release enforces — and what it does not

**The current release enforces the receipt gate, bound to the action.** An action runs only if:

- its envelope is **active** — not revoked, not expired;
- the action is **declared in the topology**;
- `cert.tlcert` + `bundle.tlbundle` **pass the external `timelayer-verifier`** (exact verdict `VALID FINAL`); and
- the receipt **attests this exact action**: the gate recomputes the envelope's
  intent commitment (`intent_scheme: "tl-intent/1"`, see `tl-agent intent-digest`)
  and passes it to the verifier via `--expect`. A receipt that is valid *in
  itself* but was issued for a different action — or an envelope edited after
  issuance — is refused (**no receipt transplant**).

**Binding policy (fail-closed at every fork).** `tl-intent/1` → recomputed
commitment. Legacy envelope with `tlsig_doc_digest` → bound to the declared
digest (weaker; cabinet compatibility). Neither → **STOP** `UNBOUND_RECEIPT`,
unless you explicitly opt out with `TL_AGENT_ALLOW_UNBOUND=1`. Unknown
`intent_scheme` → STOP. Verifier without `--expect` support → STOP.

**Not enforced yet.** Scope and policy fields (`read_only`, `network_allowed`, `write_allowed`) are **committed** into the `tl-intent/1` digest (widening them after issuance breaks the receipt match) but their *runtime semantics* are not enforced — scope enforcement is on the roadmap. `execution_log.jsonl` (`tl-agent record`) is local diagnostics, **not execution proof**: it is unsigned and mutable; a real execution receipt contour is on the roadmap.

TL-Agent is a **receipt gate and tamper-evident provenance layer, not a sandbox.**

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

Apache License 2.0
