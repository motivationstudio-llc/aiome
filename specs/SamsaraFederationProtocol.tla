---- MODULE SamsaraFederationProtocol ----
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS Nodes,           \* {n1, n2, ...}
          Hub,             \* The relay hub
          RuleIds          \* Possible rule IDs

VARIABLES node_states,     \* node_states[n] = {r1, r2, ...}
          hub_state,       \* hub_state = {r1, r2, ...}
          network_msgs     \* In-flight messages (simplified)

vars == <<node_states, hub_state, network_msgs>>

TypeOK ==
    /\ node_states \in [Nodes -> SUBSET RuleIds]
    /\ hub_state \in SUBSET RuleIds
    /\ network_msgs \in SUBSET [type: {"Push", "Pull"}, from: Nodes \cup {Hub}, rules: SUBSET RuleIds]

Init ==
    /\ node_states = [n \in Nodes -> {}]
    /\ hub_state = {}
    /\ network_msgs = {}

\* Node discovers a new rule locally
Discover(n, r) ==
    /\ r \notin node_states[n]
    /\ node_states' = [node_states EXCEPT ![n] = @ \cup {r}]
    /\ UNCHANGED <<hub_state, network_msgs>>

\* Node pushes its local state to the Hub
PushToHub(n) ==
    /\ network_msgs' = network_msgs \cup {[type |-> "Push", from |-> n, rules |-> node_states[n]]}
    /\ UNCHANGED <<node_states, hub_state>>

\* Hub processes a push and updates its global state (G-Set merge)
HubReceive(msg) ==
    /\ msg \in network_msgs
    /\ msg.type = "Push"
    /\ hub_state' = hub_state \cup msg.rules
    /\ network_msgs' = network_msgs \setminus {msg}
    /\ UNCHANGED node_states

\* Hub pulls/broadcasts global state to a node
HubBroadcast(n) ==
    /\ network_msgs' = network_msgs \cup {[type |-> "Pull", from |-> Hub, rules |-> hub_state]}
    /\ UNCHANGED <<node_states, hub_state>>

\* Node receives rules from Hub and merges (G-Set merge)
NodeReceive(n, msg) ==
    /\ msg \in network_msgs
    /\ msg.type = "Pull"
    /\ msg.from = Hub
    /\ node_states' = [node_states EXCEPT ![n] = @ \cup msg.rules]
    /\ network_msgs' = network_msgs \setminus {msg}
    /\ UNCHANGED hub_state

Next ==
    \/ \exists n \in Nodes, r \in RuleIds : Discover(n, r)
    \/ \exists n \in Nodes : PushToHub(n)
    \/ \exists msg \in network_msgs : HubReceive(msg)
    \/ \exists n \in Nodes : HubBroadcast(n)
    \/ \exists n \in Nodes, msg \in network_msgs : NodeReceive(n, msg)

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* -----------------------------------------------------------------------------
\* Properties to Verify
\* -----------------------------------------------------------------------------

\* Eventual Consistency: If no more discoveries, all nodes eventually agree with hub
EventualConsistency ==
    (ENABLED Next = FALSE) => \forall n \in Nodes : node_states[n] = hub_state

\* Safety: Rules are never lost (Grow-only property)
NoRuleLoss ==
    [][\forall n \in Nodes : node_states[n] \subseteq node_states'[n]]_vars

\* Convergence: All nodes eventually have all discovered rules
Convergence ==
    \forall r \in RuleIds : (\exists n \in Nodes : r \in node_states[n]) ~> (\forall n \in Nodes : r \in node_states[n])

====
