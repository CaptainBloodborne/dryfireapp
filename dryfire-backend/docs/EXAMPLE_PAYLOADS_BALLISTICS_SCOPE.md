# Example payloads — ballistics & scope

All endpoints require `Authorization: Bearer <token>`. Use the
`/api/v1/users/login` flow to get a token first.

## Ballistics

### POST /api/v1/ballistics/compute

Ad-hoc trajectory (everything sent inline, nothing saved).

```json
{
  "bullet": {
    "caliber_mm": 7.82,
    "weight_grain": 175.0,
    "muzzle_velocity_mps": 800.0,
    "bc_g1": 0.505
  },
  "sight": {
    "height_mm": 50.0,
    "zero_distance_m": 100.0
  },
  "atmosphere": {
    "temperature_c": 15.0,
    "pressure_hpa": 1013.25,
    "humidity": 0.0
  },
  "wind": {
    "speed_mps": 0.0,
    "from_clock": 12.0
  },
  "steps_m": [100, 200, 300, 400, 500, 600, 700, 800]
}
```

Add `?format=csv` to get a CSV body instead of JSON.

### POST /api/v1/ballistics/profiles

Create a saved profile.

```json
{
  "name": "Sako TRG-22 + Lapua 175 SMK",
  "gun_id": null,
  "ammo_id": null,
  "bullet": {
    "caliber_mm": 7.82,
    "weight_grain": 175.0,
    "muzzle_velocity_mps": 800.0,
    "bc_g1": 0.505
  },
  "sight": {
    "height_mm": 50.0,
    "zero_distance_m": 100.0
  },
  "default_atmosphere": {
    "temperature_c": 10.0,
    "altitude_m": 200,
    "humidity": 0.5
  }
}
```

### POST /api/v1/ballistics/profiles/{id}/compute

Compute using profile defaults; provide wind + steps for the shot.

```json
{
  "wind": { "speed_mps": 3.0, "from_clock": 9.0 },
  "steps_m": [100, 300, 500, 700]
}
```

You can override the profile's atmosphere by including an `atmosphere`
field (otherwise the profile's `default_atmosphere` is used):

```json
{
  "atmosphere": { "temperature_c": -10, "altitude_m": 1500, "humidity": 0.2 },
  "wind": { "speed_mps": 5.0, "from_clock": 3.0 },
  "steps_m": [100, 300, 500]
}
```

### PATCH /api/v1/ballistics/profiles/{id}

Partial update — send only fields you want to change. Sending `null`
for an optional foreign key (`gun_id`, `ammo_id`) clears it.

```json
{
  "bullet": {
    "caliber_mm": 7.82,
    "weight_grain": 180.0,
    "muzzle_velocity_mps": 790.0,
    "bc_g1": 0.520
  },
  "gun_id": null
}
```

## Scope

### POST /api/v1/scope/clicks

Click calculator. Observed POI is 4.5 cm high and 1.2 cm right at 100 m,
optic is 1/4 MOA:

```json
{
  "poi_offset_v_cm": 4.5,
  "poi_offset_h_cm": 1.2,
  "distance_m": 100.0,
  "unit": "moa",
  "click_value": { "fraction_of_unit": 0.25 }
}
```

Same observation with a 0.1 mil optic:

```json
{
  "poi_offset_v_cm": 4.5,
  "poi_offset_h_cm": 1.2,
  "distance_m": 100.0,
  "unit": "mil",
  "click_value": { "fraction_of_unit": 0.1 }
}
```

### POST /api/v1/scope/rezero

Move zero from 100 m to 200 m. The base trajectory request is the same
shape used by `/api/v1/ballistics/compute`:

```json
{
  "base_request": {
    "bullet": {
      "caliber_mm": 7.82, "weight_grain": 175.0,
      "muzzle_velocity_mps": 800.0, "bc_g1": 0.505
    },
    "sight": { "height_mm": 50.0, "zero_distance_m": 100.0 },
    "atmosphere": {
      "temperature_c": 15.0, "pressure_hpa": 1013.25, "humidity": 0.0
    },
    "wind": { "speed_mps": 0.0, "from_clock": 12.0 },
    "steps_m": [100, 200]
  },
  "current_zero_m": 100,
  "desired_zero_m": 200,
  "unit": "mil",
  "click_value": { "fraction_of_unit": 0.1 }
}
```

### POST /api/v1/scope/profiles

```json
{
  "name": "Vortex Razor HD Gen II 4.5-27x56",
  "gun_id": null,
  "unit": "mil",
  "click_value": { "fraction_of_unit": 0.1 },
  "max_elevation_units": 35.0,
  "max_windage_units": 17.0,
  "mount_height_mm": 50.0
}
```

### PATCH /api/v1/scope/profiles/{id}

```json
{
  "name": "Vortex Razor — re-mounted",
  "mount_height_mm": 48.5,
  "click_value": { "fraction_of_unit": 0.05 }
}
```
