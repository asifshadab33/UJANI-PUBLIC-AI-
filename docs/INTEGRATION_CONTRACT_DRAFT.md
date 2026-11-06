# Personal-to-Enterprise integration RFC (draft)

## Goal

Define a narrow bridge where UJANI Public AI can submit a user-approved task to
an organisation-controlled UJANI Enterprise service. The user keeps control of
their personal client; the organisation keeps control of enrolled devices,
data policy, approvals, quotas and audit records.

## Required protocol properties

1. Versioned envelope with strict unknown-field rejection.
2. Globally unique task ID, sender identity, target organisation/audience,
   issued-at and short expiry timestamps, and a cryptographically random nonce.
3. Registered operation ID only; never a shell command or arbitrary executable.
4. Digest of the exact payload bytes and a maximum payload size. The receiver must recompute the digest over the received bytes before parsing or execution and reject any mismatch.
5. Explicit data classification and minimal named access scopes.
6. User consent bound to the exact task, payload digest, destination audience
   and requested scopes.
7. Ed25519 signature over a domain-separated, canonical, length-prefixed
   encoding; key ID and audience must be signed. Receiver-enrolled keys must be
   bound to the sender identity and target organisation they are authorised for;
   both bindings must be checked against the signed task claims. Key IDs and
   audience strings are not, by themselves, proof of authorisation.
8. Authenticated confidentiality in transit for task payloads and results on
   every hop, using TLS with authenticated peers and/or equivalent end-to-end
   authenticated encryption. Signatures and payload digests provide integrity,
   not confidentiality.
9. Receiver-side key enrollment, rotation and revocation; strict signature
   verification; replay protection that survives process restarts; and worker-side
   policy re-evaluation immediately before execution. Re-evaluation must include
   current organisation policy, device status, user consent, workload class,
   resource limits and destination scope. Revoked or expired identities fail closed.
10. Signed result receipt containing task ID, input payload digest, worker identity,
    execution location, timestamps, status and output digest. The client must
    verify the receipt signature against a trusted key authorised for that worker
    and target organisation, and match the receipt task ID and input digest to
    the submitted task. Before consuming output, the client must hash the exact
    returned output bytes and compare them with the signed output digest; reject
    any signature, identity, task, input-digest or output-digest mismatch.

## Compatibility and status

UJANI Public AI currently has a version-1 signed task-envelope foundation with
short expiry, nonce, payload digest, registered operation, consent and narrow
scopes. Its verification currently depends on an already trusted public key
provided out of band and an in-memory replay guard. UJANI Enterprise has a
separate signed administrator-approval contract; that is not the same protocol.

This RFC proposes the additional audience, sender/key identity, durable replay,
worker revalidation and receipt requirements for a future interoperable
version. No Public-to-Enterprise transport or identity federation is implemented
by this document. Do not treat the two existing cryptographic structures as
wire-compatible.


## Required verification and failure behaviour

- Canonical encoding must be specified byte-for-byte and covered by cross-language
  test vectors. Parsers must reject duplicate fields, unknown versions, malformed
  encodings, oversized values and non-canonical representations.
- Key enrollment must authenticate the organisation administrator and sender
  identity out of band. Key records need an owner, organisation, allowed audiences,
  validity interval, key status and rotation/revocation history.
- Replay state must be durable and transactionally updated before accepting a
  task. Concurrent duplicate submissions and process restarts must not permit a
  second execution.
- TLS certificate and hostname verification must remain enabled. If an
  end-to-end encryption envelope is used, recipient key identity and rotation
  must be independently authenticated.
- A task must not start if policy, device posture, quota, approval, consent,
  resource availability or key status cannot be checked. No fallback to a more
  privileged route is permitted.
- Receipts must be retained with tamper-evident audit metadata. Clients must
  reject missing or unverifiable receipts where the selected operation requires
  verified execution provenance.

## Delivery status

Everything in this RFC remains a requirement for future integration work unless
a separate implementation and passing interoperability/security tests are linked
from this repository. The document itself does not implement transport,
encryption, federation, key management, durable replay storage, policy
re-evaluation, output verification or audit retention.
