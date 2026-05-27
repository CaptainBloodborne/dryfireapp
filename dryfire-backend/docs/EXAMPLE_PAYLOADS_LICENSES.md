# Example payloads — licenses

All endpoints require `Authorization: Bearer <token>`. Admin routes
additionally require `is_admin = true`.

## Bootstrap: create a license type (admin)

`POST /api/v1/licenses/admin/types`

```json
{
  "code": "storage",
  "name": "Разрешение на хранение и ношение оружия",
  "region": "RU",
  "validity_days": 1826,
  "instructions": "Подать заявление в МФЦ за 90 дней до окончания срока действия."
}
```

Response: `201 Created` with the new type, or `400 VALIDATION_FAILED`
with code `license_type with this code already exists` on duplicate.

## Register a license (user)

With auto-computed expiry (uses the type's `validity_days`):

`POST /api/v1/licenses/licenses`

```json
{
  "license_type_id": "<type_uuid>",
  "license_number": "АА-1234567",
  "issuer": "Росгвардия по Нижегородской области",
  "issued_at": "2024-06-01",
  "notes": "Original copy stored in safe.",
  "gun_ids": ["<gun_uuid_1>", "<gun_uuid_2>"]
}
```

With explicit expiry (overrides any type default):

```json
{
  "license_type_id": null,
  "license_number": "АА-1234567",
  "issuer": "Росгвардия",
  "issued_at": "2024-06-01",
  "expires_at": "2029-06-01",
  "gun_ids": []
}
```

Response includes `is_expired` and `days_until_expiry` (computed
relative to today):

```json
{
  "id": "...",
  "user_id": "...",
  "license_type_id": "...",
  "license_number": "АА-1234567",
  "issuer": "Росгвардия по Нижегородской области",
  "issued_at": "2024-06-01",
  "expires_at": "2029-06-01",
  "is_expired": false,
  "days_until_expiry": 1112,
  "notes": "Original copy stored in safe.",
  "scan_url": null,
  "gun_ids": ["...", "..."],
  "created_at": "...",
  "updated_at": "..."
}
```

Common error: trying to link a `gun_id` that doesn't belong to you →
`400 VALIDATION_FAILED` with `gun <uuid> not found or not owned by user`.

## List licenses

`GET /api/v1/licenses/licenses?expired=false&gun_id=<uuid>&q=россия&page=1&per_page=20`

Query params:
- `gun_id` — only licenses covering this gun
- `type_id` — by license type
- `expired` — `true` only expired, `false` only valid, omit for both
- `q` — substring across number / issuer / notes

## Deadlines in range

`GET /api/v1/licenses/licenses/deadlines?from=2026-01-01&to=2026-12-31`

Returns every license whose `expires_at` falls in `[from, to]`, sorted
ascending — handy for the client's calendar view.

```json
[
  {
    "license_id": "...",
    "license_number": "АА-1234567",
    "issuer": "Росгвардия",
    "expires_at": "2026-03-15"
  },
  {
    "license_id": "...",
    "license_number": "БВ-7654321",
    "issuer": "МВД",
    "expires_at": "2026-09-30"
  }
]
```

## Update a license

`PATCH /api/v1/licenses/licenses/{id}`

Send only fields to change. To detach all guns, send `"gun_ids": []`:

```json
{
  "notes": "Renewed in person at МФЦ.",
  "expires_at": "2031-06-01",
  "gun_ids": ["<gun_uuid>"]
}
```

**Important:** updating the license **resets the reminder bookkeeping**.
If a 90-day reminder was already sent for the old expiry date and the
new expiry date is now further in the future, the scheduler will
re-send a 90-day reminder when the new date approaches. That's correct
behaviour — the user changed the dates, they should be reminded again.

## Browse license types

`GET /api/v1/licenses/types?region=RU`

Returns the catalog. Useful for client dropdowns and for showing the
`instructions` text to the user.

## Reminder schedule

The scheduler ticks once per hour by default (`SCHEDULER_TICK_SECS`).
For each license that's exactly **90, 60, 45, 30, or 14 days** from
expiring, it:

1. Loads the owning user's email.
2. Sends a notification via the `Mailer` port (in dev: logs to stdout
   via `LoggingMailer`).
3. Records the send in `license_notifications` (unique on
   `(license_id, days_before)` so re-runs are idempotent).

To test the scheduler manually:

```sql
-- Make a license expire in exactly 14 days
UPDATE licenses SET expires_at = CURRENT_DATE + 14
WHERE license_number = 'АА-1234567';
DELETE FROM license_notifications WHERE license_id = (
  SELECT id FROM licenses WHERE license_number = 'АА-1234567'
);
```

Then wait at most `SCHEDULER_TICK_SECS` (or restart the app to skip
the 5 s startup delay) and check the logs for `would send verification
email`.
