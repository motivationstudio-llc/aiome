--------------------------- MODULE SamsaraKarmaProtocol ---------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC

\* -------------------------------------------------------------------------
\* Constants and Variables
\* -------------------------------------------------------------------------

CONSTANTS 
    Nodes,      \* Set of nodes in the federation (e.g., {n1, n2})
    MaxKarma    \* Limit for state space control

VARIABLES 
    karmaPool,  \* Set of all karma records created: [id, prev, owner]
    nodeStore,  \* Map of Node -> Set of karma record IDs known locally
    nodeTip,    \* Map of Node -> Latest karma record ID (the "head" of the chain)
    nextId      \* Counter for generating unique karma IDs

vars == <<karmaPool, nodeStore, nodeTip, nextId>>

\* -------------------------------------------------------------------------
\* Definitions
\* -------------------------------------------------------------------------

NONE == 0

\* -------------------------------------------------------------------------
\* Initialization
\* -------------------------------------------------------------------------

Init == 
    /\ karmaPool = {}
    /\ nodeStore = [n \in Nodes |-> {}]
    /\ nodeTip   = [n \in Nodes |-> NONE]
    /\ nextId    = 1

\* -------------------------------------------------------------------------
\* Actions
\* -------------------------------------------------------------------------

\* A node generates a new piece of "Karma" (experience/lesson)
\* It links to its current tip, creating a hash-chain-like structure.
GenerateKarma(n) == 
    /\ nextId <= MaxKarma
    /\ LET newRecord == [id |-> nextId, 
                         prev |-> nodeTip[n], 
                         owner |-> n]
       IN
       /\ karmaPool' = karmaPool \cup {newRecord}
       /\ nodeStore' = [nodeStore EXCEPT ![n] = @ \cup {nextId}]
       /\ nodeTip'   = [nodeTip EXCEPT ![n] = nextId]
       /\ nextId'     = nextId + 1

\* Two nodes synchronize their Karma stores.
\* This models the "Federation Sync" protocol.
\* In this simplified model, node n1 receives all karma from n2.
Sync(n1, n2) == 
    /\ n1 /= n2
    /\ nodeStore[n1] /= nodeStore[n1] \cup nodeStore[n2]
    /\ LET nextStore == nodeStore[n1] \cup nodeStore[n2]
       IN
       /\ nodeStore' = [nodeStore EXCEPT ![n1] = nextStore]
       /\ UNCHANGED <<karmaPool, nextId>>
       /\ IF nodeTip[n2] \in nextStore THEN
             nodeTip' = [nodeTip EXCEPT ![n1] = 
                        (LET knownIds == nextStore
                         IN IF knownIds = {} THEN NONE ELSE (CHOOSE i \in knownIds : \A j \in knownIds : i >= j))]
          ELSE
             UNCHANGED nodeTip

\* -------------------------------------------------------------------------
\* Next-State Relation
\* -------------------------------------------------------------------------

Next == 
    \/ \E n \in Nodes : GenerateKarma(n)
    \/ \E n1, n2 \in Nodes : Sync(n1, n2)

Spec == Init /\ [][Next]_vars
           /\ \A n \in Nodes : WF_vars(GenerateKarma(n))
           /\ \A n1, n2 \in Nodes : WF_vars(Sync(n1, n2))

\* IsPresent(n, id) is a helper to recursive check if all ancestors of id are in node n's store.
RECURSIVE IsPresent(_, _)
IsPresent(n, id) == 
    id = NONE \/ (id \in nodeStore[n] /\ IsPresent(n, (CHOOSE k \in karmaPool : k.id = id).prev))

\* -------------------------------------------------------------------------
\* Invariants (Safety)
\* -------------------------------------------------------------------------

\* 1. Causal Integrity: Every karma's 'prev' must exist in the pool (unless it's the first).
CausalIntegrity == 
    \A k \in karmaPool : 
        k.prev /= NONE => \E p \in karmaPool : p.id = k.prev

\* 2. Hash Chain Uniqueness: No two records have the same ID.
UniqueIds == 
    \A k1, k2 \in karmaPool : k1.id = k2.id => k1 = k2

\* 3. Fork Awareness: If a node has a tip, all its ancestors must be in its store.
AncestryCompleteness == 
    \A n \in Nodes : 
        nodeTip[n] /= NONE => IsPresent(n, nodeTip[n])

\* -------------------------------------------------------------------------
\* Temporal Properties (Liveness)
\* -------------------------------------------------------------------------

\* Eventually, all nodes should know about all generated karma
\* Use id quantification for TLC compatibility
GlobalGossip == 
    \A id \in 1..MaxKarma : 
        (id < nextId) ~> (\A n \in Nodes : id \in nodeStore[n])

=============================================================================
