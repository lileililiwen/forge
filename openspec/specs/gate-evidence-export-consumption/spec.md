# gate-evidence-export-consumption Specification

## Purpose
Forge consumes Driftwatchdog’s exported gate evidence as untrusted input, requiring attribution, evidence and revision agreement before any verdict and never granting deployment or publication.
## Requirements
### Requirement: Exported gate evidence is consumed as untrusted input

Forge SHALL parse a produced gate evidence export through bounded, single
document, vocabulary gated and revision gated handling, and SHALL classify an
unknown field name, an out-of-vocabulary state, an unsupported contract major, an
oversized or an unparseable document as an explicit refusal or an honest
unavailable outcome that leaves any previously persisted record untouched.

#### Scenario: Well-formed export consumed

- **WHEN** an export naming only recognized fields with recognized states arrives
  from an attributed producer at the captured revision
- **THEN** the record persists revision-bound with producer attribution and the
  evidence view reports its per-field states

#### Scenario: Unknown field name

- **WHEN** the document carries a field outside the consumed evidence vocabulary
- **THEN** that entry is refused naming the field, the refusal is counted, the
  remaining recognized entries stay intact, and no value is silently accepted

#### Scenario: Capability-only state used as evidence state

- **WHEN** a field carries a state the evidence vocabulary does not permit
- **THEN** the entry is refused as vocabulary misuse rather than normalised into a
  permitted state

#### Scenario: Unparseable or oversized document

- **WHEN** the producer's output cannot be parsed or exceeds the read bound
- **THEN** the surface reports unavailable quoting the producer's own bounded
  words, exits non-zero, and the prior record stays byte-identical

### Requirement: Verified states require attribution, evidence and revision agreement

Forge SHALL grant a `verified` reading only where the consumed document's run
revision equals the captured revision, the field names a resolving
project-relative evidence reference, and the producer is attributed, and SHALL
refuse a claim that contradicts its own evidence.

#### Scenario: Run revision matches captured revision

- **WHEN** the export's run revision equals the captured project revision
- **THEN** fields the producer granted as verified are reported verified and the
  freshness is fresh

#### Scenario: Revision moved after the run

- **WHEN** the working tree revision has moved past the run revision the export
  names
- **THEN** every field reads unverified with freshness stale, and a release check
  citing it is refused

#### Scenario: Verified field without resolvable evidence

- **WHEN** a field claims verified but its evidence reference does not resolve
  inside the project
- **THEN** the entry is refused and named, and is not downgraded silently into an
  unverified entry that would hide the producer error

#### Scenario: Publication claim without an artifact

- **WHEN** a document asserts publication evidence while carrying no artifact
  digest
- **THEN** the claim is refused as contradictory and no publication state is
  recorded

#### Scenario: Producer revision unknown

- **WHEN** the producer does not self-identify a revision
- **THEN** attribution records the producer name with an unknown revision and no
  claimed version

### Requirement: Evidence view is separate from the gate verdict

Forge SHALL expose consumed release evidence as its own read surface distinct
from the gate verdict surface, SHALL journal consumed evidence under the existing
gate operation kind without inventing a new kind, and SHALL not let a read or a
rehearsal persist anything.

#### Scenario: Two views agree on one run

- **WHEN** the same executed run is read as a verdict and as evidence
- **THEN** the two surfaces agree about revision, producer and timestamp while
  reporting their different content, and the verdict surface output is unchanged
  from before this capability

#### Scenario: Absent export

- **WHEN** no export has ever been consumed for the project
- **THEN** the evidence view reports absent with the reason named, every field
  unverified, and no empty field list that could be read as no evidence needed

#### Scenario: Reads journal nothing

- **WHEN** the evidence view is read repeatedly
- **THEN** the operations journal gains no rows and the persisted record stays
  byte-identical

### Requirement: Evidence findings surface without gating health

Forge SHALL report release-evidence state through a doctor finding that flags a
declaration asserting verification while carrying no evidence block, SHALL mark
that finding not applicable for projects with neither a declaration nor a consumed
export, and SHALL project it through the checker plane without altering the
emitted document schema.

#### Scenario: Assertion without evidence

- **WHEN** a declaration claims verified evidence status and carries no
  `release_evidence` block
- **THEN** the finding fails naming the inconsistency while maturity and the exit
  code stay governed by their own rules

#### Scenario: Fresh evidence backs the claim

- **WHEN** each declared verified state is backed by a fresh attributed record for
  that field
- **THEN** the finding passes and the checker emits no alert for it

#### Scenario: Project never opted in

- **WHEN** the assessed project has neither a declaration nor a consumed export
- **THEN** the finding is not applicable, the checker document stays
  byte-identical, and no surface reads the absence as health or as failure

### Requirement: Consumed evidence never grants deployment or publication

Forge SHALL keep `deployable` and publication claims outside this consumption:
consuming evidence SHALL NOT set deployability, SHALL NOT emit a release
evidence block for a generated project that has run no gate, and SHALL NOT expose
a new write surface over MCP, the API or the portal.

#### Scenario: Complete evidence still not deployable

- **WHEN** every consumed field reads verified
- **THEN** `deployment.deployable` stays whatever the project declared and the
  fleet, portal and checker report no deployability change

#### Scenario: Generated project has run nothing

- **WHEN** a project is newly generated by Forge
- **THEN** its declaration carries no `release_evidence` block and its per-profile
  generated tree stays byte-identical to the pre-capability digests

#### Scenario: Transports advertise nothing new

- **WHEN** the mature MCP tool registry, the API route table and the portal
  section set are listed after this capability exists
- **THEN** none of them gained an evidence surface

