----------------------- MODULE AiomeQuarantineProtocol -----------------------
\* The Quarantine State Machine Specification for Aiome
\* Objective: Prove that no skill can reach "Active" without passing "DryRun" successfully.

EXTENDS Naturals, Sequences, TLC

CONSTANTS Skills               \* The set of all possible skills entering the system.

VARIABLES 
    appState,                  \* A function mapping each skill to its current state.
    manifestValid              \* A function mapping each skill to a boolean (whether its semantic manifest was approved).

\* -------------------------------------------------------------------------
\* Possible States for a Skill
\* -------------------------------------------------------------------------
\* "Downloading" -> "ManifestCheck" -> "DryRunQuarantine" -> "Active"
\* If fails at any point -> "Violated"

vars == <<appState, manifestValid>>

\* Helper invariants to define valid state space
TypeOK == 
    /\ appState \in [Skills -> {"Downloading", "ManifestCheck", "DryRunQuarantine", "Active", "Violated"}]
    /\ manifestValid \in [Skills -> BOOLEAN]

\* -------------------------------------------------------------------------
\* Initial State
\* -------------------------------------------------------------------------
Init == 
    /\ appState = [s \in Skills |-> "Downloading"]
    /\ manifestValid = [s \in Skills |-> FALSE]

\* -------------------------------------------------------------------------
\* Actions (State Transitions)
\* -------------------------------------------------------------------------

\* Action 1: The download completes and it proceeds to manifest verification
DownloadComplete(s) == 
    /\ appState[s] = "Downloading"
    /\ appState' = [appState EXCEPT ![s] = "ManifestCheck"]
    /\ UNCHANGED manifestValid

\* Action 2: Check the Permission Manifest (Layer 2)
\* It randomly assigns True or False to represent either an approved or suspicious manifest.
CheckManifest(s) == 
    /\ appState[s] = "ManifestCheck"
    /\ \E verdict \in BOOLEAN :
          /\ manifestValid' = [manifestValid EXCEPT ![s] = verdict]
          /\ appState' = [appState EXCEPT ![s] = IF verdict THEN "DryRunQuarantine" ELSE "Violated"]

\* Action 3: Layer 3 Deterministic Tracer (Dry-Run Quarantine)
\* If it encounters an OOM, infinite loop, or attempts an illegal Wasmer call.
\* Simulating either success or a determined violation.
DryRunSimulate(s) == 
    /\ appState[s] = "DryRunQuarantine"
    /\ manifestValid[s] = TRUE
    /\ \E is_safe \in BOOLEAN :
          /\ appState' = [appState EXCEPT ![s] = IF is_safe THEN "Active" ELSE "Violated"]
    /\ UNCHANGED manifestValid

\* Action 4: An active skill executes
ExecuteActive(s) == 
    /\ appState[s] = "Active"
    /\ UNCHANGED vars

\* -------------------------------------------------------------------------
\* Next State Relation
\* -------------------------------------------------------------------------
Next == 
    \E s \in Skills :
       \/ DownloadComplete(s)
       \/ CheckManifest(s)
       \/ DryRunSimulate(s)
       \/ ExecuteActive(s)

Spec == Init /\ [][Next]_vars
           /\ \A s \in Skills : WF_vars(DownloadComplete(s))
                             /\ WF_vars(CheckManifest(s))
                             /\ WF_vars(DryRunSimulate(s))

\* -------------------------------------------------------------------------
\* Security Properties to be Verified by TLC Checker
\* -------------------------------------------------------------------------

\* 1. System Safety Invariant (Our Ultimate Goal)
\* It must NEVER happen that a skill is "Active" without `manifestValid` being TRUE
\* meaning it has bypassed the DryRun gate.
SafetyInvariant == 
    \A s \in Skills : appState[s] = "Active" => manifestValid[s] = TRUE

\* 2. Liveness Property
\* Every skill must eventually reach either the "Active" or "Violated" state and stay there.
Liveness == 
    \A s \in Skills : <>[](appState[s] \in {"Active", "Violated"})

=============================================================================
