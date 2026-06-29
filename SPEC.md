# TL-Agent — Product Specification

**English** · [Русский](SPEC.ru.md)

**Product:** Tamper-evident Agent Log — a tool for building governed AI agents on top of TimeLayer notarial receipts.

**Where it lives:** Personal cabinet on timelayer-os.com + a Rust SDK on GitHub.

**Implementation language:** Rust.

**Principle:** NO VALID RECEIPT → NO ACTION.

---

## Architectural decision

Two layers that never mix:

```
┌─────────────────────────────────────────┐
│  envelope.json  (agent layer)            │
│  action_id, scope, allowed_next,         │
│  status, topology_id, issued_at...       │
│                                          │
│  ┌───────────────────────────────────┐   │
│  │  cert.tlcert + bundle.tlbundle    │   │
│  │  quorum-of-nodes signature,       │   │
│  │  immutable                        │   │
│  └───────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

A notarial receipt is a **pair** of `cert.tlcert` + `bundle.tlbundle`
(exactly what the network's notarization returns); they are verified together and never touched.
`envelope.json` is the agent-side wrapper with metadata.
Together they form a **Receipt Action Unit**.

---

## Bundle structure (exchange format)

```
agent-bundle/
  manifest.json              ← bundle ID, owner, date, receipt count
  topology.json              ← graph of allowed transitions between actions
  policies/
    agent_policy.json        ← what the agent is forbidden to do
    tool_policy.json         ← which tools are allowed
    stop_policy.json         ← when the agent must stop
  receipts/
    <action_id>/
      envelope.json          ← action metadata
      cert.tlcert            ← notarial certificate (untouched)
      bundle.tlbundle        ← notarial signature bundle (verified with cert)
  exports/
    segment_01/              ← part of the topology to hand to the agent
    segment_02/
```

---

## envelope.json format

```json
{
  "tl_agent_version": "1.0",
  "receipt_id": "tlr_20260624_000001",
  "receipt_type": "permission_receipt",
  "topology_id": "topo_001",
  "action_id": "action_read_files",
  "issued_by": "user_cabinet",
  "issued_at": "2026-06-24T00:00:00Z",
  "valid_from": "2026-06-24T00:00:00Z",
  "valid_until": null,
  "status": "active",
  "previous_receipt_id": null,
  "allowed_next_actions": [
    "action_summarize",
    "action_stop_and_ask_user"
  ],
  "scope": {
    "paths": ["/project/docs"],
    "read_only": true,
    "network_allowed": false
  },
  "cert_file": "cert.tlcert",
  "bundle_file": "bundle.tlbundle",
  "action_hash_sha256": "<sha256 hex of the notarized action>"
}
```

---

## manifest.json format

```json
{
  "bundle_id": "tl_bundle_001",
  "bundle_type": "full | segment",
  "segment_id": null,
  "owner_id": "user_cabinet_id",
  "created_at": "2026-06-24T00:00:00Z",
  "topology_id": "topo_001",
  "receipt_count": 4,
  "export_mode": "portable",
  "agent_can_write": false,
  "agent_can_issue_receipts": false,
  "no_receipt_no_action": true,
  "tl_agent_version": "1.0"
}
```

---

## topology.json format

```json
{
  "topology_id": "topo_001",
  "name": "Project Review",
  "mode": "receipt-gated",
  "created_at": "2026-06-24T00:00:00Z",
  "nodes": [
    {
      "action_id": "action_read_files",
      "label": "Read project files",
      "required_receipts": ["permission_receipt", "scope_receipt"]
    },
    {
      "action_id": "action_summarize",
      "label": "Summarize findings",
      "required_receipts": ["permission_receipt"]
    },
    {
      "action_id": "action_stop_and_ask_user",
      "label": "Stop and wait",
      "required_receipts": ["stop_receipt"]
    }
  ],
  "edges": [
    {
      "from": "action_read_files",
      "to": "action_summarize",
      "condition": "result_receipt_valid"
    },
    {
      "from": "action_read_files",
      "to": "action_stop_and_ask_user",
      "condition": "always"
    }
  ]
}
```

---

## Receipt types (envelope.receipt_type)

| Type | Purpose |
|-----|-----------|
| `identity_receipt` | Who created the task |
| `task_receipt` | Recording the task |
| `permission_receipt` | Permission for an action |
| `scope_receipt` | Boundaries of an action |
| `tool_receipt` | An allowed tool |
| `execution_receipt` | The fact of execution |
| `result_receipt` | The result of an action |
| `validation_receipt` | Validation of the result |
| `stop_receipt` | Stop command |
| `revoke_receipt` | Revocation of access |
| `final_receipt` | Final state |

---

## System invariants (must not be violated)

```
INV-01: The agent does not issue valid receipts itself.
INV-02: The agent does not modify the user's bundle.
INV-03: Every action requires a permission_receipt.
INV-04: Every transition is checked against topology.json.
INV-05: Any conflict → STOP.
INV-06: Model text does not count as proof.
INV-07: PASS is impossible without a validation_receipt (when required).
INV-08: The user keeps the bundle off-platform (offline).
INV-09: .tlsig is never modified.
INV-10: envelope.json does not replace .tlsig, it only references it.
```

---

## Rust SDK — public API (plan)

```rust
// Load a bundle
let bundle = AgentBundle::load("./agent-bundle")?;

// Check before an action
match bundle.check_action("action_read_files") {
    Allow(receipt) => { /* perform the action */ }
    Stop(reason)   => { /* stop */ }
}

// Request the next allowed transition
let next = bundle.allowed_next("action_read_files")?;

// Record execution (locally, does not issue .tlsig)
bundle.record_execution("action_read_files", &result_hash)?;
```

---

## Three parts of the product

| Part | What | Where |
|-------|-----|-----|
| 1. Bundle format | JSON specification | This file |
| 2. Rust SDK | Validation library | GitHub: timelayer-agent-sdk |
| 3. Web UI | Topology builder | timelayer-os.com/cabinet |
