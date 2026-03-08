------------------------- MODULE AiomeContextEngine -------------------------
EXTENDS Naturals, FiniteSets, TLC

\* -------------------------------------------------------------------------
\* Constants and Variables
\* -------------------------------------------------------------------------

CONSTANTS 
    Sessions

VARIABLES 
    state,        \* State of the Context Engine itself
    compacting,   \* Boolean flag indicating if compaction is currently running
    activeSessions\* Set of currently active session IDs

vars == <<state, compacting, activeSessions>>

\* -------------------------------------------------------------------------
\* Definitions
\* -------------------------------------------------------------------------

States == {"Uninitialized", "Ready", "Disposed"}

(* 
Context Engine Lifecycle:
1. Uninitialized
2. Ready (after bootstrap)
3. Disposed (after dispose)
*)

\* -------------------------------------------------------------------------
\* Initialization
\* -------------------------------------------------------------------------

Init == 
    /\ state = "Uninitialized"
    /\ compacting = FALSE
    /\ activeSessions = {}

\* -------------------------------------------------------------------------
\* Actions
\* -------------------------------------------------------------------------

\* System calls bootstrap on the Context Engine
Bootstrap(s) == 
    /\ state = "Uninitialized"
    /\ s \in Sessions
    /\ state' = "Ready"
    /\ activeSessions' = activeSessions \cup {s}
    /\ UNCHANGED compacting

\* System asks to ingest a new message
Ingest(s) == 
    /\ state = "Ready"
    /\ ~compacting
    /\ s \in activeSessions
    /\ UNCHANGED vars

\* System triggers a compaction for a session
Compact(s) == 
    /\ state = "Ready"
    /\ ~compacting
    /\ s \in activeSessions
    /\ compacting' = TRUE
    /\ UNCHANGED <<state, activeSessions>>

\* Compaction finishes
CompactDone == 
    /\ compacting
    /\ compacting' = FALSE
    /\ UNCHANGED <<state, activeSessions>>

\* System shuts down the Context Engine
Dispose == 
    /\ state \in {"Ready", "Uninitialized"}
    /\ state' = "Disposed"
    /\ UNCHANGED <<compacting, activeSessions>>

\* -------------------------------------------------------------------------
\* Next-State Relation
\* -------------------------------------------------------------------------

Next == 
    \/ \E s \in Sessions : Bootstrap(s)
    \/ \E s \in Sessions : Ingest(s)
    \/ \E s \in Sessions : Compact(s)
    \/ CompactDone
    \/ Dispose
    \/ (state = "Disposed" /\ UNCHANGED vars)

Spec == Init /\ [][Next]_vars /\ WF_vars(Dispose) /\ WF_vars(CompactDone)

\* -------------------------------------------------------------------------
\* Invariants (Safety)
\* -------------------------------------------------------------------------

\* 1. Engine cannot receive ingest calls if disposed
TypeSafety == 
    state = "Disposed" => ~ENABLED(Ingest("any"))

\* 2. Compaction blocks new ingests (Strict ordering)
CompactMutex == 
    compacting => ~ENABLED(Ingest("any"))

\* -------------------------------------------------------------------------
\* Temporal Properties (Liveness)
\* -------------------------------------------------------------------------

\* Eventually the engine will be Disposed
Liveness == 
    <>(state = "Disposed")

=============================================================================
