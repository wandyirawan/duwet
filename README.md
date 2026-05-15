# 🫐 Duwet — Warehouse TUI

Terminal-based warehouse management for Salad Buah ERP. Keyboard-driven, runs on potato PCs.

```
┌─ Duwet ───────────────────────────────────────────┐
│ Stock In [0] | Stock Out [1] | Check [2]           │
│────────────────────────────────────────────────────│
│ ┌─ SKU ────────┐  ┌─ Quantity ──┐  ┌─ Warehouse ┐ │
│ │ PRD-001      │  │ 10          │  │ 1          │ │
│ └──────────────┘  └─────────────┘  └────────────┘ │
│ ┌─ Reference ────────────────────────────────────┐ │
│ │ PO-2026-001                                    │ │
│ └────────────────────────────────────────────────┘ │
│              [ Stock In ]                          │
│ OK: +10 PRD-001                                    │
└────────────────────────────────────────────────────┘
```

## Stack

- **Rust** — Single binary, zero dependencies on target machine
- **Ratatui** — Terminal UI framework
- **Tokio** — Async runtime for HTTP calls
- **reqwest** — HTTP client to Salak + Mangosteen

## How It Works

```
Duwet TUI ──Login──→ Salak (:8000) POST /auth/login → Mangosteen
    │
    ├─ Stock In ──→ Salak (:8000) POST /stock-in
    ├─ Stock Out ─→ Salak (:8000) POST /stock-out
    └─ Check ────→ Salak (:8000) GET /inventory?sku=X
```

## Quick Start

```bash
# Clone & build
git clone https://github.com/wandyirawan/duwet.git
cd duwet

# Configure
cp .env.example .env
# Edit: SALAK_URL, MANGOSTEEN_URL

# Run (dev)
make dev

# Build binary (for deployment)
make build
# → target/release/duwet
```

## Deployment to Warehouse PC

```bash
# Build on dev machine
make build

# Copy to warehouse PC (potato PC, no Rust needed)
scp target/release/duwet .env user@warehouse-pc:~/duwet/

# On warehouse PC
cd ~/duwet
# Edit .env to point to actual servers
./duwet
```

## Configuration

`.env`:

```env
SALAK_URL=http://192.168.1.10:8000
```

## Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | Submit form |
| `Esc` | Clear / Normal mode |
| `1/2/3` | Switch tab |
| Type | Input text |

## Tabs

### Stock In (1)
- SKU, Quantity, Warehouse ID, Reference
- POST to Salak `/stock-in`

### Stock Out (2)
- SKU, Quantity, Warehouse ID, Reference
- POST to Salak `/stock-out`

### Check (3)
- SKU lookup
- GET Salak `/inventory?sku=X`
- Shows product ID, name, current stock

## License

MIT

---

Part of **Salad Buah** — Pick the fruits you need. Skip the rest.
