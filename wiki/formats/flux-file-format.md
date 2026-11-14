# Flux file storage: `.flx` and `.tflx`

Design date: 2026-11-14. Source baseline: `41bf95b`.
Status: version 1.0 design with an initial bounded implementation in [flux/file.rs](../../src/flux/file.rs) and sketch integration. The initial codec stages owned values, supports files up to 16 MiB, rejects more than 100,000 values per region or nesting beyond 64, and writes through a sibling temporary file. General streaming descriptor import, configurable limits, inspect/verify/conversion facades, and optional newer-version header extensions remain planned.

Execution plan: [FLX/TFLX interface implementation](../plans/flux-file-interface-implementation-plan.md), with proposed public APIs, compatibility changes, delivery phases, and acceptance gates.

Use **`.flx` for compact binary storage** and **`.tflx` for equivalent editable text**. Both contain the same typed Flux value, the same application schema, and the same document metadata. Applications choose their schema through the preamble rather than inventing a new storage envelope for each module.

## 1. Fit to the existing Flux interfaces

The source interfaces in [fluxexport.rs](../../src/flux/fluxexport.rs) and [fluximport.rs](../../src/flux/fluximport.rs) currently describe null, bool, unsigned 64-bit integer, 64-bit float, UTF-8 string, array, and object. `Str` and `String` differ in ownership, not stored meaning. `FluxSource`, `FluxSink`, `BoxedSink`, `ExpectedType`, and `ObjValidated` are adapter mechanisms, not additional wire types.

Version 1 stores exactly that value model. Signed integers, raw byte blobs, packed numeric arrays, references, compression, encryption, and indexes require separately designed extensions. Do not silently serialize pointers, Rust enum layouts, native struct padding, or buffer capacity. An array contains its logical elements only.

The existing JSON output is a diagnostic/export representation. It converts non-finite floats to null, does not explicitly preserve integer-versus-float identity, and suppresses write errors. Consequently it cannot serve as the lossless `.tflx` codec unchanged. Keep existing JSON behavior available and add dedicated fallible file codecs.

## 2. Shared logical envelope and usual file preamble

Every stored document contains:

```text
Identification and storage version
Required feature bits
Document metadata
One typed payload value
Integrity and completeness information
```

Separate the **storage version**, initially `1.0`, from the **application schema version**. An update to sketch entity fields need not change the Flux container version. A text/binary conversion preserves document identity, document revision, timestamps, schema, and ordered payload values; it is not a new domain edit.

### Required metadata

Metadata is a Flux object encoded through the same value codec as the payload. These members are required and appear in this order in canonical writer output:

| Member | Flux type | Meaning and validation |
| --- | --- | --- |
| `Schema` | string | Nonempty application schema identifier, e.g. `trellis.sketch` or `trellis.example`. Case-sensitive; never a filename or code-loading instruction. |
| `SchemaVersion` | u64 | Positive application schema version. The schema owner defines compatibility and migrations. |
| `DocumentId` | string | Stable opaque 128-bit identity written as exactly 32 lowercase hexadecimal digits; all-zero identity is invalid. A new document receives a new random identity; Save As preserves it unless explicitly saving a new independent copy. |
| `DocumentRevision` | u64 | Application-managed revision; nonnegative. It is not a file offset, wall clock, or the storage format version. |
| `CreatedUtc` | string or null | Original creation time, or null when unknown. |
| `ModifiedUtc` | string or null | Most recent application content/metadata edit time, or null when unknown. Transcoding alone preserves it. |
| `Producer` | object | Required string members `Name` and `Version`, identifying the writer software; both nonempty. Conversion updates Producer to identify the actual writer. |

Timestamps use `YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`: exactly nine fractional digits, UTC, valid Gregorian calendar dates in years 0001-9999, hours 00-23, minutes/seconds 00-59. Do not store local timezone-dependent timestamps. Precision lower than a nanosecond is padded with zeros; a timestamp is descriptive metadata, not a uniqueness guarantee.

Readers accept metadata members in any order, reject duplicates, and validate all required members and types. Writers emit the listed order. Optional member `Extensions` is an object of namespaced application-owned metadata, e.g. `"trellis.ui": {...}`. Preserve unknown metadata on generic conversion. The schema decides whether domain-specific information is valid; units, workplanes, and entity data belong in the payload.

Checksums detect accidental corruption; they do not establish trust, authorship, or authenticity. Files contain no embedded executable instructions, required external paths, or automatic network dependencies.

## 3. Binary `.flx` envelope

All fixed-width integers are **little-endian**. The file has three adjacent regions, without implicit alignment padding:

```text
[64-byte fixed header][MetadataBytes bytes of metadata][PayloadBytes bytes of payload]
```

### Fixed header, version 1.0

| Byte offset | Bytes | Field | Required interpretation |
| --- | --- | --- | --- |
| 0 | 8 | Magic | Hex `54 52 46 4c 58 0d 0a 1a`, byte string `TRFLX\r\n\x1a`. |
| 8 | 2 | FormatMajor | `1`. |
| 10 | 2 | FormatMinor | `0`. |
| 12 | 4 | HeaderBytes | `64` in version 1.0. |
| 16 | 4 | RequiredFeatures | `0` in version 1.0; unknown set bits reject the file. |
| 20 | 4 | HeaderCrc32c | CRC-32C of all `HeaderBytes`, with bytes 20-23 treated as zero. |
| 24 | 8 | FileBytes | Exact total file byte count. |
| 32 | 8 | MetadataBytes | Encoded metadata length, greater than zero. |
| 40 | 8 | PayloadBytes | Encoded payload length, greater than zero; null still occupies one byte. |
| 48 | 4 | MetadataCrc32c | CRC-32C of the exact encoded metadata region. |
| 52 | 4 | PayloadCrc32c | CRC-32C of the exact encoded payload region. |
| 56 | 8 | Reserved | All zero. |

Metadata begins at `HeaderBytes`; payload begins at `HeaderBytes + MetadataBytes`. Checked arithmetic must establish:

```text
FileBytes = HeaderBytes + MetadataBytes + PayloadBytes
```

Reject truncation, trailing bytes, inconsistent lengths, checksum mismatch, or nonzero reserved bytes. Neither an all-zero checksum nor a zero length means "unknown". There is no footer or concatenation of documents in version 1.0. A complete valid header, both bounded regions, and matching checksums establish completeness.

The on-disk byte lengths are `u64` because files can outgrow a single in-memory container. Project array indexes/counts remain `u32`. The first codec may impose a lower supported file-size limit, but must reject an oversized file explicitly before narrowing or allocating.

### CRC-32C definition

Use reflected polynomial `0x82f63b78`, initial accumulator `0xffffffff`, and final XOR `0xffffffff`. Process every byte from least-significant bit first; no implicit zero bytes, newline normalization, or omitted terminators. The check value for ASCII `123456789` is `0xe3069283`; the empty byte sequence has checksum zero. Serialize checksum fields as little-endian u32. An optimized implementation must agree with the bitwise reference and these vectors.

### Binary value encoding

| Tag byte | Value | Bytes following the tag |
| --- | --- | --- |
| `00` | null | None. |
| `01` | false | None. |
| `02` | true | None. |
| `03` | u64 | Unsigned variable-length integer as defined below. |
| `04` | f64 | Eight bytes of the float's exact bit pattern, little-endian. |
| `05` | string | Variable-length UTF-8 byte count followed by exactly those bytes. |
| `06` | array begin | Zero or more complete values followed by tag `08`. |
| `07` | object begin | Zero or more string-key/value pairs followed by tag `08`. |
| `08` | container end | Valid only where an array/object can end. |
| `09`-`ff` | reserved | Reject in version 1.0. |

Objects contain a string value (including tag `05`) for each key, immediately followed by its value. Keys may be empty but may not repeat within an object. Strings are valid UTF-8; embedded NUL is permitted and counted. No terminator or Unicode normalization is applied. Object member order and array order are preserved. Each metadata/payload region contains exactly one complete value; metadata's root must be an object. Root scalars and empty containers are valid payloads.

Unsigned variable-length encoding uses seven value bits per byte, least-significant group first; bit 7 indicates that another byte follows. Use the shortest possible encoding. Zero is `00`, 127 is `7f`, 128 is `80 01`, and `u64::MAX` is nine `ff` bytes followed by `01`. Reject overlong forms, more than ten bytes, a tenth byte larger than `01`, or unterminated input. Length encodings use the same rule but must also fit configured limits and remaining bytes.

Container end tags fit the existing callback-based export interfaces: a producer need not know an array's length before it starts. This allows one pass over stateful callbacks and avoids materializing an entire document just to write collection counts. Generic skipping of an unknown object member walks one complete nested value under the same limits; there is no unbounded scan or special treatment of unknown scalar tags.

Small encoding examples, including delimiters:

```text
null                  00
u64(42)               03 2a
f64(1.0)              04 00 00 00 00 00 00 f0 3f
"A"                   05 01 41
[true, null]          06 02 00 08
{"X": u64(42)}        07 05 01 58 03 2a 08
```

## 4. Text `.tflx` envelope and grammar

Text is UTF-8 without BOM. Canonical writers use LF, two-space indentation, and a final newline; readers also accept CRLF. The first physical line is exactly `TFLX 1.0` followed by LF or CRLF. Version detection reads this bounded ASCII line before parsing the body.

```text
TFLX 1.0
Features u64(0);
Metadata {
  "Schema": "trellis.example",
  "SchemaVersion": u64(1),
  "DocumentId": "00112233445566778899aabbccddeeff",
  "DocumentRevision": u64(1),
  "CreatedUtc": null,
  "ModifiedUtc": null,
  "Producer": {"Name": "Trellis", "Version": "0.1.0"}
};
Data {
  "Count": u64(42),
  "X": f64(1.0),
  "Label": "hello",
  "Flags": [true, false, null]
};
Integrity none;
End;
```

The example explicitly omits checksums for hand editing. Normal application writers emit `Integrity crc32c(0xMMMMMMMM, 0xPPPPPPPP);`, where the first argument checks metadata and the second checks payload as defined below. The [paired reference files](examples/minimal.tflx) include actual checksums and a matching [binary file](examples/minimal.flx).

Informal EBNF, after the first version line:

```text
file       = "Features", uint, ";",
             "Metadata", object, ";",
             "Data", value, ";",
             "Integrity", ("none" | "crc32c", "(", hex32, ",", hex32, ")"), ";",
             "End", ";", EOF ;
value      = "null" | "false" | "true" | string | uint | float | floatbits | array | object ;
uint       = "u64", "(", decimalUnsigned, ")" ;
float      = "f64", "(", decimalFloat, ")" ;
floatbits  = "f64bits", "(", "0x", 16 * hexDigit, ")" ;
array      = "[", [value, {",", value}, [","]], "]" ;
object     = "{", [string, ":", value, {",", string, ":", value}, [","]], "}" ;
hex32      = "0x", 8 * hexDigit ;
```

Whitespace is allowed between tokens. Outside strings, `#` starts a comment through the line ending. Keywords and type names are case-sensitive. Hexadecimal digits may be either case on input; canonical output uses lowercase. No bare numbers, unquoted keys, expressions, includes, macros, duplicate keys, or implicit type conversions are allowed. The value of `Features` must fit u32 and obey the same rules as binary `RequiredFeatures`.

String literals use double quotes, escapes `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\u` followed by four hexadecimal digits. Unescaped U+0000-U+001F are invalid. Decode valid paired surrogate escapes to one Unicode scalar and reject unpaired surrogates. All other valid Unicode scalars may appear literally. Escape decoding does not normalize strings. Byte counts for equivalent binary strings refer to decoded UTF-8 bytes, not textual escape lengths.

`decimalUnsigned` is one or more ASCII digits, no sign, no separators, within u64 range; leading zeroes are accepted but not written. `decimalFloat` is `[+-]?digits[.digits]?[e[+-]?digits]?`, with `e` or `E`, digits required after any decimal point/exponent, and a finite result after correctly rounded conversion to f64 (nearest, ties to even). Preserve negative zero. Reject overflow to infinity; ordinary rounded subnormal/underflow results are permitted. `f64(1)` is a float, whereas `u64(1)` is an integer.

Canonical finite-float output uses the shortest decimal significand that parses to the original bits; among equally short candidates choose the one nearest the exact binary value, breaking an exact tie with an even final significand digit. Add an explicit `.0` when there is no decimal point or exponent. Use lowercase `e`, no positive sign or redundant exponent zeroes, and the fixed form when the decimal exponent is from -6 through 20, scientific form otherwise. Negative zero is `f64(-0.0)`. An implementation must centralize and golden-test that formatting policy; ordinary default `Display` is not a format specification.

`f64bits(0xhhhhhhhhhhhhhhhh)` denotes the entire u64 bit pattern, with the most significant digit printed first. It is accepted for all floats and is the canonical form for infinity and NaN, preserving sign and payload bits. For example, positive infinity is `f64bits(0x7ff0000000000000)` and one NaN is `f64bits(0x7ff8000000000001)`. Codecs must not perform floating arithmetic on such values; use bit conversion to avoid changing NaN payloads. A domain such as geometry can reject non-finite values even though generic Flux storage supports them.

### Text integrity and equivalence

Let `B(V)` mean the binary encoding in section 3 of the decoded typed value `V`, using minimal variable-length integers and preserving member order and exact float bits. Text checksum arguments are `CRC32C(B(Metadata))` and `CRC32C(B(Data))` respectively. Comments, whitespace, indentation, line endings, and equivalent string/float spellings therefore do not affect checksums. Reordering object members does affect them because generic conversion preserves producer order.

Text does not store physical byte lengths or a binary header CRC: these are derived binary-envelope details, and including them would make ordinary text edits brittle. The required preamble, bounded parser, `End;`, and EOF validate textual structure/completeness. When `Integrity none;` is explicitly present, report integrity as **not checked**, rather than treating the file as corrupt or verified. A stale supplied checksum is always an error; no silent fallback to unchecked mode. Applications may require checksums by policy.

Conversion preserves the typed value tree, including null versus missing, bool versus number, u64 versus f64, signed zero, all f64 bits, strings, member order, and array order. It may change formatting and Producer, and must then recompute checksums/header fields. It does not preserve comments, whitespace, borrowing/ownership, in-memory capacity, or wire byte identity. Do not round-trip through `JsonOutStream` or infer types with `TryPostParsed`.

### Paired reference vector

The example pair contains the metadata and payload shown above, with checksums enabled. Expected binary envelope values:

| Field | Value |
| --- | --- |
| FileBytes | 293 |
| HeaderBytes | 64 |
| MetadataBytes | 180 |
| PayloadBytes | 49 |
| HeaderCrc32c | `0x8b9efacb` |
| MetadataCrc32c | `0xc2705c6c` |
| PayloadCrc32c | `0x28e39626` |

Its complete 64-byte header is:

```text
0000: 54 52 46 4c 58 0d 0a 1a 01 00 00 00 40 00 00 00
0010: 00 00 00 00 cb fa 9e 8b 25 01 00 00 00 00 00 00
0020: b4 00 00 00 00 00 00 00 31 00 00 00 00 00 00 00
0030: 6c 5c 70 c2 26 96 e3 28 00 00 00 00 00 00 00 00
```

These metadata checksums describe Producer `Trellis` version `0.1.0`; changing Producer during conversion intentionally changes the metadata/header checksums while leaving the payload checksum unchanged.

## 5. Reading, validation, compatibility, and safe file writes

### Detection and compatibility

Recognize the magic/version line first; use the extension for file dialogs and expected-format validation, not as the sole content detector. An extension/magic disagreement produces a clear diagnostic; an explicit import/conversion action may accept a renamed file. Do not fall back from corrupt binary to text or guess that arbitrary JSON is `.tflx`.

Major-version changes are incompatible. Future minor versions may add optional metadata or optional fixed-header extension bytes, while preserving the initial 64-byte layout and existing value grammar. They must not repurpose tags or add required meaning without a required feature bit. A reader may accept a newer minor only if it understands every required feature and can safely skip optional additions. Bound `HeaderBytes` by 64 KiB and check its CRC over the entire header; unknown optional extension bytes are included in that CRC. Version 1.0 requires exactly 64 bytes. The binary metadata/payload bounds formula remains unchanged.

Unknown application fields are schema policy: inspect/convert modes preserve them, tolerant schema import may skip complete values, and strict import rejects them. Unknown value tags or malformed skipped values always fail. `SchemaVersion` is checked by the application before publishing a domain document. Reject unsupported schemas without instantiating domain factories from strings.

### Limits and validation order

Initial default reader limits, configurable by a trusted caller:

- Header: 64 KiB; encoded metadata: 1 MiB; total input file: 1 GiB.
- Decoded string/key: 16 MiB; nesting depth: 128 containers.
- Per-container children/members: 1,000,000; total decoded values, including object keys and metadata: 10,000,000.
- Aggregate decoded allocation budget: 512 MiB, including text escape buffers, field-name indexes, staging values, and schema objects. Bound temporary scratch independently; do not promise the input-size limit guarantees allocation success.

These are implementation defaults, not representational limits. Binary readers check magic, bounded header size, header CRC, version/features, reserved fields, checked sizes, and configured budgets before allocation. Check metadata checksum and schema metadata before processing payload. Validate both region checksums and exact consumption before success. Text readers validate the version and Features first, then metadata/schema, payload, integrity declaration, End, and EOF. An iterative parser tracks container kind and whether an object expects a key or value; errors include byte offset, field path, and text line/column when applicable.

Do not commit partial imports into a live model. File-level import constructs a fresh owned staging object, performs descriptor/container validation, verifies the complete envelope and application invariants, then swaps it into the session. Existing callbacks can mutate their targets during parsing; `ObjValidated` alone is not a transaction for an arbitrary object graph. In-place import must require a caller-provided rollback mechanism, and is not the default file API.

Both reader paths must handle `ExpectedType` by exact string comparison and call `ObjValidated` exactly once after a complete accepted object. Resolve source adapters with a bounded expansion count so a cyclic `FluxSource` cannot recurse forever. Post values using the typed `TryPost...` methods. Null is posted only to a target/sink that accepts null; `FieldImp::Null` used to skip a field is a separate parser decision. A rejected array callback/capacity is an error, never permission to ignore extra elements or panic. Import into fresh containers to prevent stale trailing elements from a previous value.

For strings, an owned file reader supports `FieldImp::String` and copying sinks. It must reject a long-lived borrowed `FieldImp::Str` unless an explicitly lifetime-bound buffered API can prove the decoded bytes outlive the result. Never borrow from refillable buffers or temporary unescape storage, and never leak strings to manufacture lifetimes.

### Writing and finalization

1. Capture immutable application payload/metadata and its revision. Write to a uniquely named sibling temporary file created exclusively, not to the destination.
2. For binary, reserve the header, stream metadata and payload once while counting bytes and computing CRCs, flush buffered output, then seek and write the final complete header and its checksum. Do not replay stateful export callbacks to precompute lengths.
3. For text, emit the preamble and typed values, computing CRCs from the same emitted logical events, then emit Integrity and End. Default application output includes checksums.
4. Explicitly flush, finalize, and sync the temporary file. Propagate write, flush, seek, sync, and close/finalization errors; destructor flushing is not evidence of success.
5. Replace the destination using supported platform semantics, preserving the previous file if replacement fails. Do not delete the old destination to make rename succeed. Sync the parent directory where supported for rename durability; distinguish atomic visibility from power-loss durability, and do not claim guarantees on arbitrary network filesystems.
6. Only successful completion marks the saved snapshot's history state as saved. If the user edited meanwhile, the current document remains dirty. On failure keep the session dirty and preserve the previous destination; temporary-file cleanup must stay within the selected directory.

The version 1 binary file writer requires `Write + Seek`. A non-seekable output may stage to a bounded temporary file and copy the completed result, or report unsupported; no unknown-length sentinel is introduced. A text writer can use non-seekable output directly. For fallible export, stop traversing producer callbacks after the first write error.

## 6. Implementation plan in Trellis

Keep generic envelopes/codecs in Flux and application schema mapping in the owning module. In particular, Fleck's sketch persistence supplies a Flux source/import target; it does not own the general `.flx` header implementation. No Serde or Tokio dependency is needed.

| Step | Files and responsibilities | Acceptance gate |
| --- | --- | --- |
| 1: contracts and errors | New `flux/file.rs`: `FluxFileMetadata`, `FluxFileInfo`, `FluxReadLimits`, `FluxFileError`, format selection, detection, and checked preamble parsing. Re-export through `flux/mod.rs`. | Header layout/length arithmetic, schema metadata validation, and magic/version failures match golden vectors. |
| 2: binary codec | New `flux/binary.rs`: streaming typed writer/reader, CRC accounting, bounded skipping, exact float bits, and seek-based final header. | Golden `.flx` round trips; callback traversal occurs once; malformed tags/varints/containers fail with offsets. |
| 3: text codec | New `flux/text.rs`: typed tokenizer/parser and deterministic writer using project storage. Use Shard only if doing so preserves dependency direction; do not introduce Flux-to-Shard while Shard already uses Flux. | `.flx -> .tflx -> .flx` preserves value bits and metadata apart from explicitly updated Producer; formatting and checksums are stable. |
| 4: typed import adapter | Shared dispatch over `FieldImp`, source resolution, null/sinks, `ExpectedType`, container completion validation, and budget accounting. | Overflow/type errors, borrowed-string rejection, unknown fields, callback capacity, and validation failures are fallible and leave the live model unchanged. |
| 5: file lifecycle | Safe save/load APIs, sibling-temp lifecycle, overwrite integration, explicit finalization, and conversion entry points. | Injected short-write/flush/seek/sync/replace failures preserve the prior file and session state. |
| 6: first application | Fleck maps the sketch schema; Fascia recognizes both extensions and dispatches by schema; generic conversion remains independent of sketch code. | New sketch -> binary save -> text conversion -> edit -> reopen retains analytic geometry and document identity. |

Use private operational fields and project `Arr`/`Buff`/`Stash`, sorted key indexes, `USeg` traversal, `u32` logical counts and `u32::MAX` invalid handles. Keep `u64` where disk byte lengths or the wire value model require it. Avoid a per-value heap node when streaming descriptors suffice; generic lossless conversion may use bounded staging where necessary to update metadata before emission.

### Existing interface gaps to address explicitly

- `IFluxExportSink::DispatchFieldExp` returns `()`. Add a small fallible export interface or an internal fallible dispatcher plus `Finish() -> Result`. A legacy adapter must latch the first error, and the public file API must always check it. Do not break existing diagnostic JSON call sites to add file storage.
- `IStream::At` returns zero beyond EOF; `BuffStream` suppresses some read errors and exposes cached size through `Size`. These are unsuitable as the only storage-validation signals. Use a dedicated fallible byte cursor over `std::io::Read` with u64 position and bounded project buffers. Preserve existing parser APIs until a separate migration is justified.
- `OutStream::Position`/`SetPosition` describe its buffer marker and do not implement filesystem seek. Explicitly flush and use a real seekable file adapter for header backpatching. Never seek underneath pending buffered output.
- Existing `Arr`/`Buff` import adapters can assert/panic on capacity exhaustion, and import descriptors lack general array completion/count hooks. Introduce bounded fallible adapters or extend container contracts before advertising arbitrary existing descriptors as safe file-import targets. Fixed-size arrays need exact cardinality checks; record required-field validation is schema responsibility.
- `FluxError` currently has only syntax/type/EOF/overflow. Keep compatibility or wrap it in `FluxFileError` with I/O cause, checksum mismatch, unsupported format/schema/feature, limit exceeded, invalid metadata, duplicate key, capacity/cardinality, validation failure, and location context.

### Verification

Register tests with `jeeves_test!` in `src/flux/_tests.rs`. Required cases include the reference pair, every primitive/root kind, empty/nested collections, escaped Unicode/NUL, maximum u64, negative zero, subnormals, infinities, distinct NaN bit patterns, malformed/overlong varints, duplicate/empty keys, truncated values/headers, bad lengths/CRCs, trailing bytes, unsupported versions/features, allocation/depth/count limits, unknown-field skipping, failed validation, fixed-array sizes, and write/finalization failures. Test both text integrity modes and whitespace-only edits with unchanged checksums. Test truncation at every byte boundary of the small golden binary and deterministic mutation cases without unbounded fuzz allocations.

Run narrow Flux tests and a clean offline build for each implementation slice; include Shard regressions if shared descriptor contracts change. The paired example files are design vectors verified independently of the future codecs, not evidence that native storage has already been implemented.
