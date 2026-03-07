# Magento Cache Invalidation Contract

VeloServe provides a Magento-compatible invalidation contract at:

- `POST /api/v1/cache/invalidate`

## Request Validation Rules

- Method must be `POST`
- `Content-Type` must be `application/json`
- Unknown custom `x-*` headers are rejected
- Payload uses strict schema validation (`deny_unknown_fields`)
- Domain, path, and tags are normalized before execution

## Payload Shapes

### 1) URL or path purge

```json
{
  "scope": "url",
  "domain": "shop.example.com",
  "paths": ["/", "/category/*", "/product/sku-123"]
}
```

Notes:
- `*` suffix on a path means prefix purge (bounded by fan-out limits)
- Paths are normalized (leading slash, duplicate slashes collapsed)

### 2) Tag purge

```json
{
  "scope": "tag",
  "tags": ["product:sku-123", "path:shop.example.com/category/shoes"]
}
```

### 3) Tag-group purge (bulk)

```json
{
  "scope": "tag_group",
  "groups": [
    {
      "name": "catalog",
      "tags": ["product:sku-123", "product:sku-124"]
    }
  ]
}
```

## Safety Controls

- Idempotency dedupe window: 15 seconds
- Rate guard: 120 invalidation requests per 60-second window
- Bounded fan-out:
  - max 128 targets per request
  - max 32 groups
  - max 64 tags per group

Idempotency keys can be sent via:
- request header `x-idempotency-key`
- payload field `idempotency_key`

## Observability

Each invalidation logs structured fields:
- `request_id`
- `scope`
- `affected_keys`
- `latency_ms`
- `outcome` (`ok`, `deduped`, `rate_limited`)

Response includes:

```json
{
  "success": true,
  "request_id": "inv-123456",
  "scope": "tag",
  "deduped": false,
  "affected_keys": 4,
  "outcome": "ok"
}
```
