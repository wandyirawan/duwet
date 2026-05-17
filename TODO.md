# Duwet — Next: Full Salak API Coverage

## Target
All Salak endpoints accessible from Duwet TUI. Full CRUD for warehouse operators.

## Progress (2026-05-17)

### ✅ Done Today
| # | Tab | Status | Notes |
|---|-----|--------|-------|
| 0 | Stock In | ✅ | Existing |
| 1 | Stock Out | ✅ | Existing |
| 2 | Check | ✅ | Existing |
| 3 | Transactions | ✅ | Read-only table, scrollable |
| 4 | Categories | ✅ | Full CRUD: browse/create/edit/delete |

### ✅ Salak Side (API updates)
| Endpoint | Status |
|----------|--------|
| PUT /warehouses/{id} | ✅ added |
| GET /warehouses/{id} | ✅ added |
| DELETE /warehouses/{id} | ✅ added (409 on FK) |
| PUT /categories/{id} | ✅ added |
| GET /categories/{id} | ✅ added |
| DELETE /categories/{id} | ✅ added (409 on FK) |
| DELETE /products/{id} | ✅ added (409 on FK) |
| Migration 003 | ✅ categories FK SET NULL → RESTRICT |

## Remaining

### [3] Products — Table + Search (🆕 NEXT)
- GET /products?search=X&category_id=Y
- Render as scrollable table: SKU | Name | Price | Category | Active
- Search bar at top (type to filter)
- Enter on row → detail view → Edit (PUT /products/{id})
- `n` key → New product form (POST /products)

### [5] Warehouses — Table + CRUD
- GET /warehouses → table: ID | Name | Location
- `n` → new warehouse
- Enter → edit

## Technical notes

- Table widget: ratatui table with scrolling (TableState + with_offset)
- Search: input field at top, debounced GET on Enter
- Form: same pattern as stock-in/out forms + Categories CRUD pattern
- All API calls use existing `app.token` from login
- Error handling: show status bar at bottom
