# VeloServe WordPress Plugin (v1)

This directory contains the first shippable iteration of the VeloServe WordPress plugin.

## Layout

- `veloserve-cache/`: installable plugin source
- `tests/`: lightweight flow tests (activation, settings persistence, registration success/failure)

## Packaging

Create a distributable zip:

```bash
cd wordpress-plugin
zip -r veloserve-cache.zip veloserve-cache
```

## Tests

```bash
wordpress-plugin/tests/run-tests.sh
```

The test runner requires a local `php` binary.
