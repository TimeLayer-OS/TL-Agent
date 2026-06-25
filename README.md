# TL-Agent

Receipt-gated control plane for AI agents, built on [TimeLayer](https://timelayer-os.com) notarial receipts.

## Core principle

**NO VALID RECEIPT → NO ACTION**

An agent cannot authorize its own actions and cannot rewrite the history of what it did. Every permitted action is backed by a `.tlsig` receipt issued by the TimeLayer network before execution begins.

---

## What's in this repo

| Path | Description |
|------|-------------|
| `timelayer-agent-sdk/` | Rust library + `tl-agent` CLI binary |
| `example-bundle/` | Ready-to-run example agent bundle with real `.tlsig` receipts |
| `SPEC.md` | Full specification of bundle formats and invariants |

---

## Quick start

### 1. Download `tl-agent`

Grab the latest binary from [Releases](../../releases).

```bash
chmod +x tl-agent
./tl-agent --help
```

### 2. Check an action before running it

```bash
tl-agent check ./example-bundle action_read_files
# ALLOW
#   action  : action_read_files
#   receipt : tlr_20260624_000001
#   next    : action_summarize
```

Exit code `0` = ALLOW, `1` = STOP. Wire this into any agent or shell script.

### 3. Audit the whole bundle

```bash
tl-agent audit ./example-bundle
# === tl-agent audit ===
# actions : 4  valid: 4  stop: 0
# result: ALL VALID — bundle is ready for use
```

### 4. Record execution

```bash
DIGEST=$(sha256sum output.txt | awk '{print $1}')
tl-agent record ./example-bundle action_read_files "$DIGEST"
# recorded → execution_log.jsonl
```

---

## Bundle structure

```
agent-bundle/
  manifest.json          ← bundle ID, owner, receipt count
  topology.json          ← allowed transitions between actions
  policies/
    agent_policy.json    ← what the agent may not do
    tool_policy.json     ← which tools are allowed
    stop_policy.json     ← when the agent must halt
  receipts/
    <action_id>/
      envelope.json      ← agent metadata (references proof.tlsig)
      proof.tlsig        ← notarial receipt — never modified
  exports/
    segment_01/          ← partial bundle for handing to the agent
```

`.tlsig` is a notarial document issued by the TimeLayer network. It is never modified by TL-Agent — only read and verified.

---

## CLI commands

```
tl-agent check  <bundle_dir> <action_id>              Gate check. Exit 0=ALLOW, 1=STOP.
tl-agent next   <bundle_dir> <action_id>              List allowed next actions per topology.
tl-agent audit  <bundle_dir>                          Check every action in the bundle.
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
# → target/release/tl-agent
```

The CLI has no runtime dependencies beyond `timelayer-verifier` (included in releases).

---

## Verifier

Receipt verification is performed offline by `timelayer-verifier`. Place it:
- Next to `tl-agent` (recommended)
- In `bundle/bin/`
- Anywhere on `PATH`

Or pass `--verifier /path/to/timelayer-verifier` explicitly.

---

## Security model

- **Fail-closed**: any error (missing receipt, invalid signature, unknown action) → STOP
- **Two layers**: `proof.tlsig` (notarial, immutable) + `envelope.json` (agent metadata, references `.tlsig`)
- **No self-authorization**: the bundle is built and receipts are issued before the agent runs
- **Tamper-evident history**: `execution_log.jsonl` records what ran, with output digests

---

## License

MIT
