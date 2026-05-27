# Example payloads — ammo

All endpoints require `Authorization: Bearer <token>`. Admin routes
additionally require `is_admin = true`.

## Bootstrap: create an ammo type (admin)

`POST /api/v1/ammo/admin/types`

```json
{
  "manufacturer": "Lapua",
  "name": "Scenar 175gr",
  "caliber": ".308 Win",
  "bullet_type": "match",
  "projectile_type": "centerfire",
  "powder_charge_grain": null,
  "bullet_weight_grain": 175.0,
  "notes": "Factory match-grade load."
}
```

Returns `201` with the new type. Duplicate `(manufacturer, name, caliber)`
returns `400 VALIDATION_FAILED` with `ammo_type ... already exists`.

## Acquire ammo (user)

`POST /api/v1/ammo/transactions`

Bought 100 rounds:

```json
{
  "ammo_type_id": "<ammo_type_uuid>",
  "delta": 100,
  "note": "Picked up at the range store."
}
```

Response includes the new ledger row **and** the resulting stock:

```json
{
  "transaction": {
    "id": "...",
    "user_id": "...",
    "ammo_type_id": "...",
    "gun_id": null,
    "delta": 100,
    "occurred_at": "2026-05-25T07:30:00Z",
    "note": "Picked up at the range store.",
    "created_at": "2026-05-25T07:30:00Z"
  },
  "resulting_stock": {
    "user_id": "...",
    "ammo_type_id": "...",
    "quantity": 100,
    "updated_at": "2026-05-25T07:30:00Z"
  }
}
```

## Consume at the range

```json
{
  "ammo_type_id": "<ammo_type_uuid>",
  "gun_id": "<sako_trg_uuid>",
  "delta": -42,
  "occurred_at": "2026-05-24T11:00:00Z",
  "note": "Zero session at 100 m, then a few groups at 300."
}
```

Trying to consume more than you have:

```json
{ "ammo_type_id": "...", "delta": -999 }
```

→ `400 VALIDATION_FAILED` with code `insufficient stock — consumption would go below zero`.
The transaction does **not** land — both the ledger and stock stay
consistent.

## Browse history

`GET /api/v1/ammo/transactions?gun_id=<sako_uuid>&direction=consume&from=2026-01-01T00:00:00Z&to=2026-12-31T23:59:59Z&page=1&per_page=50`

Direction values: `acquire`, `consume`. Omit for both.

## Current stocks

`GET /api/v1/ammo/stocks?caliber=.308%20Win`

```json
{
  "items": [
    {
      "user_id": "...",
      "ammo_type": {
        "id": "...",
        "manufacturer": "Lapua",
        "name": "Scenar 175gr",
        "caliber": ".308 Win",
        "bullet_type": "match",
        "projectile_type": "centerfire",
        "powder_charge_grain": null,
        "bullet_weight_grain": 175.0,
        "notes": "Factory match-grade load.",
        "created_at": "...",
        "updated_at": "..."
      },
      "quantity": 58,
      "updated_at": "2026-05-24T11:00:00Z"
    }
  ],
  "page": 1,
  "per_page": 20,
  "total": 1
}
```

By default zero-quantity rows are hidden. Pass `?include_zero=true` to
see ammo types you used to own but ran through.

## Statistics

### By caliber

`GET /api/v1/ammo/stats/by-caliber?from=2026-01-01T00:00:00Z`

(`to` is optional; omitted defaults to "now")

```json
[
  { "caliber": ".22 LR",   "acquired": 1500, "consumed":  720, "net":  780 },
  { "caliber": ".308 Win", "acquired":  300, "consumed":  142, "net":  158 },
  { "caliber": "9x19",     "acquired":  200, "consumed":  200, "net":    0 }
]
```

### By gun

`GET /api/v1/ammo/stats/by-gun?from=2026-01-01T00:00:00Z&to=2026-06-30T23:59:59Z`

```json
[
  { "gun_id": "<sako_uuid>",   "rounds_fired": 142, "last_used_at": "2026-05-24T11:00:00Z" },
  { "gun_id": "<glock_uuid>",  "rounds_fired": 200, "last_used_at": "2026-04-15T16:00:00Z" }
]
```

### Usage over time

`GET /api/v1/ammo/stats/usage?bucket=week&from=2026-04-01T00:00:00Z&to=2026-05-31T23:59:59Z&caliber=.308%20Win`

```json
[
  { "bucket": "2026-03-30", "acquired":   0, "consumed":   0 },
  { "bucket": "2026-04-06", "acquired": 100, "consumed":   0 },
  { "bucket": "2026-04-13", "acquired":   0, "consumed":  42 },
  { "bucket": "2026-04-20", "acquired":   0, "consumed":   0 },
  { "bucket": "2026-04-27", "acquired":  50, "consumed":   0 },
  { "bucket": "2026-05-04", "acquired":   0, "consumed":  20 },
  { "bucket": "2026-05-11", "acquired":   0, "consumed":   0 },
  { "bucket": "2026-05-18", "acquired":   0, "consumed":  80 },
  { "bucket": "2026-05-25", "acquired":   0, "consumed":   0 }
]
```

The response contains a row for **every bucket in the range**, even empty
ones — easier to plot than dealing with gaps. The `bucket` is the start
of the period (Postgres `date_trunc` semantics — weeks start on Monday).

Limits:
- `from` must be on or before `to`.
- `bucket=day` is rejected for ranges over 5 years (use `week` or `month`).

## Browse the ammo type catalog (any authenticated user)

`GET /api/v1/ammo/types?caliber=.308%20Win&bullet_type=match`

Filter options: `manufacturer` (substring), `caliber` (exact),
`bullet_type`, `projectile_type`, `powder_charge_min/max`, and the
free-text `q` for substring search across manufacturer/name/notes.
