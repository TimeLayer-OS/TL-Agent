# TL-Agent — Спецификация продукта

**Продукт:** Tamper-evident Agent Log — инструмент для построения управляемых AI-агентов на основе нотариальных квитанций TimeLayer.

**Где живёт:** Личный кабинет на timelayer-os.com + Rust SDK на GitHub.

**Язык реализации:** Rust.

**Принцип:** NO VALID RECEIPT → NO ACTION.

---

## Архитектурное решение

Два слоя, которые никогда не смешиваются:

```
┌─────────────────────────────────────────┐
│  envelope.json  (агентский слой)         │
│  action_id, scope, allowed_next,         │
│  status, topology_id, issued_at...       │
│                                          │
│  ┌───────────────────────────────────┐   │
│  │  proof.tlsig  (нотариальная)      │   │
│  │  подпись кворума нод, неизменна  │   │
│  └───────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

`.tlsig` — нотариальный документ, не трогается никогда.
`envelope.json` — агентская обёртка с метаданными.
Вместе они образуют **Receipt Action Unit**.

---

## Структура bundle (формат обмена)

```
agent-bundle/
  manifest.json              ← ID bundle, владелец, дата, кол-во квитанций
  topology.json              ← граф допустимых переходов между действиями
  policies/
    agent_policy.json        ← что агенту запрещено делать
    tool_policy.json         ← какие инструменты разрешены
    stop_policy.json         ← когда агент обязан остановиться
  receipts/
    <action_id>/
      envelope.json          ← метаданные действия
      proof.tlsig            ← нотариальная квитанция (нетронутая)
  exports/
    segment_01/              ← часть топологии для выдачи агенту
    segment_02/
```

---

## Формат envelope.json

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
  "tlsig_file": "proof.tlsig",
  "tlsig_doc_digest": "<blake3 hex из .tlsig>",
  "content_digest": "<blake3 hex самого envelope.json без этого поля>"
}
```

---

## Формат manifest.json

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

## Формат topology.json

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

## Типы квитанций (envelope.receipt_type)

| Тип | Назначение |
|-----|-----------|
| `identity_receipt` | Кто создал задачу |
| `task_receipt` | Фиксация задачи |
| `permission_receipt` | Разрешение на действие |
| `scope_receipt` | Границы действия |
| `tool_receipt` | Разрешённый инструмент |
| `execution_receipt` | Факт выполнения |
| `result_receipt` | Результат действия |
| `validation_receipt` | Проверка результата |
| `stop_receipt` | Команда остановки |
| `revoke_receipt` | Отзыв допуска |
| `final_receipt` | Финальное состояние |

---

## Инварианты системы (нельзя нарушать)

```
INV-01: Агент не выпускает валидные квитанции сам.
INV-02: Агент не изменяет bundle пользователя.
INV-03: Каждое действие требует permission_receipt.
INV-04: Каждый переход проверяется по topology.json.
INV-05: Любой конфликт → STOP.
INV-06: Текст модели не считается доказательством.
INV-07: PASS невозможен без validation_receipt (если требуется).
INV-08: Пользователь хранит bundle вне платформы (офлайн).
INV-09: .tlsig не модифицируется никогда.
INV-10: envelope.json не заменяет .tlsig, только ссылается.
```

---

## Rust SDK — публичный API (plan)

```rust
// Загрузить bundle
let bundle = AgentBundle::load("./agent-bundle")?;

// Проверить перед действием
match bundle.check_action("action_read_files") {
    Allow(receipt) => { /* выполнить действие */ }
    Stop(reason)   => { /* остановиться */ }
}

// Запросить следующий допустимый переход
let next = bundle.allowed_next("action_read_files")?;

// Зафиксировать выполнение (локально, не выдаёт .tlsig)
bundle.record_execution("action_read_files", &result_hash)?;
```

---

## Три части продукта

| Часть | Что | Где |
|-------|-----|-----|
| 1. Формат bundle | JSON-спецификация | Этот файл |
| 2. Rust SDK | Библиотека валидации | GitHub: timelayer-agent-sdk |
| 3. Web UI | Topology builder | timelayer-os.com/cabinet |
