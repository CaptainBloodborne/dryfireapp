# Example payloads — laws

All endpoints require `Authorization: Bearer <token>`. Admin routes
additionally require `is_admin = true`.

## Bootstrap: create a category (admin)

`POST /api/v1/laws/admin/categories`

```json
{
  "code": "storage",
  "name": "Хранение оружия",
  "parent_id": null,
  "sort_order": 1
}
```

## Create a law (admin / ingester)

`POST /api/v1/laws/admin/laws`

```json
{
  "law_key": "fz-150-1996",
  "title": "ФЗ-150 «Об оружии»",
  "summary": "Основной федеральный закон, регулирующий оборот гражданского, служебного и боевого оружия в Российской Федерации.",
  "body": "Глава 1. Общие положения...\n\nСтатья 1. Основные понятия...",
  "region": "RU",
  "category_id": "<category_uuid>",
  "tags": ["storage", "carry", "license"],
  "effective_at": "1996-12-13"
}
```

Response: `201 Created` with the new law (`current_version = 1`).
Duplicate `law_key` → `400 VALIDATION_FAILED` with `law with this law_key already exists`.

## Update a law (auto-snapshots prior version)

`PATCH /api/v1/laws/admin/laws`

Note: identified by `law_key` in the body, not by URL.

```json
{
  "law_key": "fz-150-1996",
  "title": "ФЗ-150 «Об оружии»",
  "summary": "Обновлённый текст с учётом изменений 2024 года.",
  "body": "...",
  "region": "RU",
  "category_id": "<category_uuid>",
  "tags": ["storage", "carry", "license", "penalty"],
  "effective_at": "2024-08-01"
}
```

When body/title/summary/tags/category/effective_at change, the DB
trigger:
1. Inserts a row into `law_versions` with the *prior* state.
2. Increments `current_version`.

So immediately after this PATCH, `current_version = 2` and the v1
snapshot is queryable via `/laws/{id}/versions`.

## List / filter

`GET /api/v1/laws/laws?region=RU&any_tags=storage,carry&page=1&per_page=20`

Query params:
- `region` — exact match.
- `category_id` — by category.
- `any_tags=a,b,c` — OR semantics (any of these tags).
- `all_tags=a,b` — AND semantics (must have all).
- `effective_after=YYYY-MM-DD` — laws effective on or after this date.
- `updated_after=ISO-8601` — laws updated after this timestamp.

Unknown tag names in the CSV are silently dropped (permissive reads).

## Full-text search

`GET /api/v1/laws/laws/search?q=хранение%20патронов&region=RU&any_tags=storage`

The `q` parameter supports PostgreSQL `websearch_to_tsquery` syntax:

- Plain words: `хранение патронов` — AND between terms.
- Quoted phrases: `"короткоствольное оружие"`.
- OR: `охота OR хранение`.
- Negation: `оружие -игрушка`.

Response:

```json
{
  "items": [
    {
      "law": {
        "id": "...",
        "law_key": "fz-150-1996",
        "title": "ФЗ-150 «Об оружии»",
        "summary": "Основной федеральный закон...",
        "body": "...",
        "region": "RU",
        "category_id": "...",
        "tags": ["storage", "carry", "license"],
        "current_version": 2,
        "effective_at": "2024-08-01",
        "created_at": "...",
        "updated_at": "..."
      },
      "rank": 0.198,
      "snippet": "…условия <mark>хранения</mark> огнестрельного оружия и <mark>патронов</mark> к нему…"
    }
  ],
  "page": 1,
  "per_page": 20,
  "total": 7
}
```

Empty `q` → `400 VALIDATION_FAILED`. Query >512 chars → same.

## What changed since I last visited?

`GET /api/v1/laws/laws/changes`

No params → uses the calling user's `last_visit_at`, falling back to
account `created_at` if the user has never visited (first session).
Region also defaults to the user's profile region.

`GET /api/v1/laws/laws/changes?since=2026-05-01T00:00:00Z&region=RU&per_page=50`

Explicit overrides supported. Use case: a "What's new since you opened
the app on May 1st" notification badge.

## Revision history

`GET /api/v1/laws/laws/{id}/versions`

```json
[
  {
    "id": "...",
    "law_id": "<law_uuid>",
    "law_key": "fz-150-1996",
    "version": 1,
    "title": "ФЗ-150 «Об оружии»",
    "summary": "Прежняя версия...",
    "body": "...",
    "tags": ["storage", "carry", "license"],
    "category_id": "...",
    "effective_at": "2023-01-01",
    "snapshot_at": "2024-08-01T09:00:00Z"
  }
]
```

Sorted by `version DESC` so the newest historical snapshot is first.
The *current* version (the row in `laws`) is not duplicated here —
to get the full picture, combine `GET /laws/{id}` and
`GET /laws/{id}/versions`.

## Categories

`GET /api/v1/laws/categories`

```json
[
  { "id": "...", "code": "storage",   "name": "Хранение оружия",    "parent_id": null, "sort_order": 1, ... },
  { "id": "...", "code": "transport", "name": "Транспортировка",     "parent_id": null, "sort_order": 2, ... },
  { "id": "...", "code": "hunting",   "name": "Охотничье оружие",    "parent_id": null, "sort_order": 3, ... }
]
```

## Background ingestion

The HTTP layer exposes the admin endpoints any ingester needs to:
1. List existing laws (to know what we already have).
2. POST a new law when a feed publishes one not yet in DB.
3. PATCH an existing law when the feed reports an update — this
   automatically snapshots the prior version into history.

A scheduled ingester job (parallel to the license-reminder scheduler)
would fetch from `pravo.gov.ru` / ConsultantPlus / external API, diff
against current state, and call these endpoints with an admin token.
That's left for a separate iteration — the data model and write API
are complete.

## Tag vocabulary

Fixed set (validated at the DB layer via CHECK constraint):
`transport`, `carry`, `storage`, `hunting`, `self_defense`, `sport`,
`license`, `inheritance`, `inspection`, `penalty`, `other`.
