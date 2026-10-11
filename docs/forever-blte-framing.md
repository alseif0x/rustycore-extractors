# Pure BLTE framing inspector

`blte::framing::inspect(encoded, expected_content_size, blte::Limits)` accepts
exactly one complete supplied BLTE span and returns all-or-error redacted
`Evidence`. It performs no allocation, I/O, key lookup, decryption or inflation.
The existing decoder is unchanged apart from exporting this separate module.

For framed objects it validates the signature, complete exact header/table,
frame-count ceiling, checked frame extents, EOF and sum of declared content
sizes against the independent expected size. Every encoded-frame MD5 must
match, including an all-zero expected MD5. A valid first encrypted wrapper does
not stop inspection of later frames. `N` additionally checks payload length;
`Z` only identifies the outer mode and does not inspect zlib bytes or termination.
`E` validates outer name-size 0/8, nonce-size 4/8, Salsa20 selector `S` and
nonempty ciphertext. It neither interprets nor returns identifiers/nonces, nor
validates the encrypted inner mode or bytes. `F` and unknown modes reject.

Evidence copies only form, encoded/expected sizes, optional table-declared size,
frame count, N/Z/E counts and verified frame-MD5 count. It returns no digest,
key, nonce, payload, plaintext or ciphertext bytes. The existing encoded-size,
decoded-declaration and frame-count limits apply; they are caller-owned ceilings.
There is no output allocation, descriptor vector or per-frame retained state.
Work is linear in the bounded supplied encoded span and descriptor count.

`Form::HeaderlessUnhashed` explicitly has no frame checksum and no table size
declaration. Its sole outer frame extends to EOF. Extra bytes inside a `Z` or
`E` payload cannot be distinguished from opaque interior bytes without decoding;
framing success makes no interior-termination claim. Neither form authenticates
an independent EKey, CKey, complete plaintext or ciphertext interior. This is
diagnostic structural evidence, without a key-only diagnosis or full extraction
claim. Invalid objects return no partial evidence.

## Future exact-span QA composition

A separate route owner would retain its privately selected archive entry and
guarded handle, admit the entire encoded record using existing store limits,
check the complete captured archive extent and read exactly that record. After
removing the known 30-byte local prelude, it would call `inspect` on the whole
remaining BLTE span with independently known decoded size, then recheck held and
named identities before returning redacted evidence. Short reads or mismatches
remain terminal, with no prefix fallback or alternative-route substitution.
The current metadata route API has no byte adapter; this composition is a plan,
not an executed LocalStore integration or real-client acceptance. Independent
encoded-key/content-key authentication and decoding remain separate gates.
