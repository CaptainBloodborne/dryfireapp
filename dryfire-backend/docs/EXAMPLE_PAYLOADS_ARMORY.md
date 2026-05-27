# Example payloads — armory

All endpoints require `Authorization: Bearer <token>`. Admin routes
additionally require the calling user to have `is_admin = true` in DB.

## Register a gun

`POST /api/v1/armory/guns`

```json
{
  "manufacturer": "Sako",
  "model": "TRG-22",
  "class": "rifle",
  "caliber": ".308 Win",
  "serial": "ABC1234567",
  "date_of_purchase": "2024-06-12",
  "photo_url": null,
  "notes": "Bought from official dealer."
}
```

Or with a catalog reference (autofill on the client side):

```json
{
  "catalog_id": "f7b2…",
  "manufacturer": "Sako",
  "model": "TRG-22",
  "class": "rifle",
  "caliber": ".308 Win",
  "serial": "ABC1234567",
  "date_of_purchase": "2024-06-12"
}
```

Response: `201 Created` with `GunResponse` — note the `serial` is
**not** returned, only `serial_last4`:

```json
{
  "id": "…",
  "user_id": "…",
  "manufacturer": "Sako",
  "model": "TRG-22",
  "class": "rifle",
  "caliber": ".308 Win",
  "serial_last4": "4567",
  "date_of_purchase": "2024-06-12",
  "photo_url": null,
  "notes": "Bought from official dealer.",
  "created_at": "…",
  "updated_at": "…"
}
```

## List guns

`GET /api/v1/armory/guns?class=rifle&caliber=.308%20Win&q=sako&page=1&per_page=20&sort=-purchase`

Query params (all optional):
- `class` — filter by class: `rifle`, `shotgun`, `pistol`, `revolver`, `smg`, `other`
- `caliber` — exact caliber match
- `q` — substring search across manufacturer, model, notes (case-insensitive)
- `page`, `per_page` — pagination, defaults 1 / 20, max per_page = 100
- `sort` — `created_at`, `updated_at`, `purchase`, `manufacturer`, `model`, `caliber`. Prefix with `-` for descending.

Response shape:

```json
{
  "items": [ { "id": "...", ... }, ... ],
  "page": 1,
  "per_page": 20,
  "total": 3
}
```

## Reveal serial

`GET /api/v1/armory/guns/{id}/serial`

No body. Audited as `gun.serial_reveal`.

```json
{
  "id": "…",
  "serial": "ABC1234567"
}
```

## Update a gun

`PATCH /api/v1/armory/guns/{id}`

Send only the fields you want to change. Use `null` on an optional
field to clear it:

```json
{
  "notes": "Resold to friend, keeping record.",
  "photo_url": null
}
```

Change the serial (re-encrypted on write):

```json
{ "serial": "ZZZ9999999" }
```

## Delete

`DELETE /api/v1/armory/guns/{id}` → `204 No Content`.

## Browse the catalog

`GET /api/v1/armory/catalog?class=rifle&caliber=.308%20Win&q=trg`

Same pagination conventions as `/guns`.

## Admin — create a catalog entry

`POST /api/v1/armory/admin/catalog`

```json
{
  "manufacturer": "Sako",
  "model": "TRG-22",
  "class": "rifle",
  "caliber": ".308 Win",
  "barrel_length_mm": 660,
  "weight_g": 4700,
  "capacity": 10,
  "notes": "Finnish bolt-action precision rifle."
}
```

Response: `201 Created` with the catalog entry, or `400 VALIDATION_FAILED`
with code `manufacturer+model already exists` on a duplicate.

## Admin — update / delete

```
PATCH /api/v1/armory/admin/catalog/{id}
DELETE /api/v1/armory/admin/catalog/{id}
```

A non-admin caller hitting these routes gets `403 ADMIN_REQUIRED`.

## How to make a user an admin

There's no admin-creation endpoint yet (deliberate — bootstrap is a
manual step). For now:

```sql
UPDATE users SET is_admin = TRUE WHERE login = 'jdoe';
```
