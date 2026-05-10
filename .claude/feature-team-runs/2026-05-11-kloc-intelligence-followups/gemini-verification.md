# Gemini provider verification

Date: 2026-05-10
Refactor commit: TBD (per-operation provider config)

## Summary

| Path | Native Gemini | OpenRouter (default) |
| --- | --- | --- |
| LLM chat (`explain`, `enrich`) | ✅ works | ✅ works |
| Embeddings (`enrich`, `search`) | ❌ Haystack `usage` parse error | ✅ works |

**Recommended setup**: Native Gemini for the LLM, OpenRouter for embeddings (you can still pick `google/gemini-embedding-001` on OpenRouter — the protocol bridge fills in the missing `usage` field that breaks Haystack).

## Verified provider info

- OpenAI-compatible base URL: `https://generativelanguage.googleapis.com/v1beta/openai/`
- LLM model: `gemini-3-flash-preview` — confirmed via Google's docs and OpenRouter's catalog
- Embedding model: `gemini-embedding-001` — native dimension 3072 (Matryoshka-truncatable to 1536 or 768)

## Smoke test results

### Chat (LLM)

Direct OpenAI SDK + Haystack `OpenAIChatGenerator` both call Gemini's `/v1beta/openai/chat/completions` successfully:

```
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
LLM_API_KEY=$GEMINI_API_KEY
LLM_MODEL=gemini-3-flash-preview
```

Returns a valid reply. The first test reply was truncated by `max_tokens=100`; bumping it produced full output. No format quirks; the Haystack pipeline output is normal.

Note: Haystack `ChatPromptBuilder` uses Jinja2 templates which Gemini chat handles correctly via the OpenAI-compat layer.

### Embeddings — native Gemini endpoint (BROKEN with Haystack)

Symptom:

```
PipelineRuntimeError: Component name: 'embedder' / Component type: 'OpenAIDocumentEmbedder'
TypeError: 'NoneType' object is not iterable
```

Root cause: Gemini's OpenAI-compat embeddings response **omits the `usage` field**. Verified with curl:

```bash
curl https://generativelanguage.googleapis.com/v1beta/openai/embeddings ...
# response: {"object": "list", "data": [...], "model": "..."}   ← no `usage`
```

Haystack's `OpenAIDocumentEmbedder._embed_batch` does `dict(response.usage)` (haystack 2.21+). When `usage` is `None`, this crashes. This is a Haystack/protocol-shim issue, not a kloc-intelligence issue.

The OpenAI Python SDK itself accepts the response (it represents `usage` as `None`). Only the Haystack wrapper assumes usage is always present.

### Embeddings — OpenRouter (works)

Verified with both qwen/qwen3-embedding-8b (default) AND google/gemini-embedding-001 routed through OpenRouter:

```
EMBEDDING_API_URL=https://openrouter.ai/api/v1
EMBEDDING_API_KEY=$OPENROUTER_KEY
EMBEDDING_MODEL=google/gemini-embedding-001
EMBEDDING_DIMENSION=3072
```

OpenRouter normalizes the response to include `usage`, so Haystack works. Returns 3072-dim embeddings.

## Workarounds for native Gemini embeddings

If you want to call Gemini's native embeddings endpoint without OpenRouter, options are:

1. **Patch Haystack** (upstream issue): add a defensive check around `response.usage`. Trivial 2-line fix in `OpenAIDocumentEmbedder._embed_batch` that hasn't been merged upstream yet.
2. **Use OpenRouter as the embedding gateway** (recommended) — preserves Haystack compatibility.
3. **Use the OpenAI SDK directly** for embeddings (bypassing Haystack) and write the rows to Qdrant manually. This is a non-trivial deviation from the current pipeline and out of scope here.

Documented as Limitation #1 in the kloc-intelligence overview doc.

## Mixed-provider config (the use case this refactor enables)

Recommended `.env` for someone with a Gemini quota who wants OpenRouter only for embeddings:

```
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
LLM_API_KEY=<gemini-key>
LLM_MODEL=gemini-3-flash-preview

EMBEDDING_API_URL=https://openrouter.ai/api/v1
EMBEDDING_API_KEY=<openrouter-key>
EMBEDDING_MODEL=google/gemini-embedding-001
EMBEDDING_DIMENSION=3072
```

If you change `EMBEDDING_DIMENSION`, drop the existing Qdrant collections and re-run `enrich`:

```bash
uv run python -c "from qdrant_client import QdrantClient; QdrantClient(url='http://localhost:6335').delete_collection('code_embeddings'); QdrantClient(url='http://localhost:6335').delete_collection('explain_embeddings')"
uv run kloc-intelligence enrich
```

## Cost notes (May 2026 retail pricing)

- `gemini-3-flash-preview`: $0.50 / 1M input, $3.00 / 1M output (Gemini API)
- `gemini-embedding-001`: $0.15 / 1M input (Gemini API)
- OpenRouter typically adds a small spread plus its 5% fee.

For the kloc-reference-project-php (~50 enrichable nodes), full enrichment costs single-digit cents on either provider.

## Action items

- [ ] (optional) File a PR upstream against `deepset-ai/haystack` for the null-usage fix. Maintains Gemini compatibility for everyone.
- [x] Document mixed-provider env config in `docs/usage/kloc-intelligence.md` and `.env.example`.
