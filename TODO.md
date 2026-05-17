# Duwet — Next: Full Salak API Coverage

## Target
All Salak endpoints accessible from Duwet TUI. Full CRUD for warehouse operators.

## Menu Structure (tabs to add)

```
[0] Stock In     — POST /stock-in              ✅ done
[1] Stock Out    — POST /stock-out             ✅ done
[2] Check        — GET /inventory?sku=X        ✅ done
[3] Products     — GET /products (table)        🆕
[4] Categories   — GET /categories (table)      ✅ done ✅
[5] Warehouses   — GET /warehouses (table)      🆕
[6] Transactions — GET /inventory/transactions  ✅ done ✅
```

## Per-tab details

### [3] Products — Table + Search
- GET /products?search=X&category_id=Y
- Render as scrollable table: SKU | Name | Price | Category | Active
- Search bar at top (type to filter)
- Enter on row → detail view → Edit (PUT /products/{id})
- `n` key → New product form (POST /products)

### [4] Categories — Table + CRUD ✅ done
- GET /categories → scrollable table: ID | Name | Slug
- `n` → new category form
- Enter → edit
- `d` → delete confirm dialog
- Auto slug generation on backend

### [5] Warehouses — Table + CRUD
- GET /warehouses → table: ID | Name | Location
- `n` → new warehouse
- Enter → edit

### [6] Transactions — Table (read-only) ✅ done
- GET /inventory/transactions?product_id=X&limit=50
- Table: Time | SKU | Delta | Reference | Warehouse
- Scrollable, most recent first
- Up/Down arrows to scroll

## Technical notes

- Table widget: use ratatui table with scrolling (ListState + offset)
- Search: input field at top, debounced GET on Enter
- Form: same pattern as existing stock-in/out forms
- All API calls use existing `app.token` from login
- Error handling: show status bar at bottom

## API reference (from Salak)

| Method | Endpoint | Tab |
|--------|----------|-----|
| GET | /products?search=&category_id= | 3 |
| GET | /products/{id} | 3 detail |
| POST | /products | 3 new |
| PUT | /products/{id} | 3 edit |
| GET | /categories | 4 |
| POST | /categories | 4 new |
| GET | /warehouses | 5 |
| POST | /warehouses | 5 new |
| GET | /inventory/transactions | 6 |
