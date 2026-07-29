# ApriReader language package format

Language packages use `.apridict` or `.apripkg` extensions and ZIP
compression. The archive root contains `manifest.json` plus an exact,
kind-specific payload. Nested paths and additional payload names are rejected.

## Common manifest fields

- `schemaVersion`: `1`
- `id`, `version`, `name`
- `kind`: `dictionary` or `translation`
- `sourceLanguage` and optional `targetLanguage`
- `sourceUrl`: HTTPS origin of the data or model
- `licenseSpdx`: a license allowed by the dependency policy
- `attribution`: required credit or provenance
- `engine`, `engineVersion`
- `files`: payload path, byte size, and lowercase SHA-256

## Dictionary package

The engine is `aprireader-dictionary-v1`. The only payload is `entries.json`,
an array of:

```json
{
  "term": "quiet",
  "definitions": ["making little noise"],
  "examples": ["a quiet library"]
}
```

## Translation package

The engine is `onnxruntime-text-v1`. The only payload is a hashed `model.onnx`.
`inputName` and `outputName` identify one-dimensional string tensors. The model
must accept one UTF-8 string and return one translated UTF-8 string.
`targetLanguage` is required. ONNX Runtime is supplied by ApriReader; packages
cannot include or load native libraries.

This deliberately narrow contract avoids executing tokenizers, scripts, native
helpers, or arbitrary package code. Models with token-ID or custom-operator
interfaces require a future reviewed engine adapter and are rejected by this
version.
