---- MODULE AiomeContextEngine_TTrace_1772960605 ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, AiomeContextEngine

_expression ==
    LET AiomeContextEngine_TEExpression == INSTANCE AiomeContextEngine_TEExpression
    IN AiomeContextEngine_TEExpression!expression
----

_trace ==
    LET AiomeContextEngine_TETrace == INSTANCE AiomeContextEngine_TETrace
    IN AiomeContextEngine_TETrace!trace
----

_inv ==
    ~(
        TLCGet("level") = Len(_TETrace)
        /\
        compacting = (FALSE)
        /\
        activeSessions = ({})
        /\
        state = ("Disposed")
    )
----

_init ==
    /\ state = _TETrace[1].state
    /\ activeSessions = _TETrace[1].activeSessions
    /\ compacting = _TETrace[1].compacting
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ state  = _TETrace[i].state
        /\ state' = _TETrace[j].state
        /\ activeSessions  = _TETrace[i].activeSessions
        /\ activeSessions' = _TETrace[j].activeSessions
        /\ compacting  = _TETrace[i].compacting
        /\ compacting' = _TETrace[j].compacting

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("AiomeContextEngine_TTrace_1772960605.json", _TETrace)

=============================================================================

 Note that you can extract this module `AiomeContextEngine_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `AiomeContextEngine_TEExpression.tla` file takes precedence 
  over the module `AiomeContextEngine_TEExpression` below).

---- MODULE AiomeContextEngine_TEExpression ----
EXTENDS Sequences, TLCExt, Toolbox, Naturals, TLC, AiomeContextEngine

expression == 
    [
        \* To hide variables of the `AiomeContextEngine` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        state |-> state
        ,activeSessions |-> activeSessions
        ,compacting |-> compacting
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_stateUnchanged |-> state = state'
        
        \* Format the `state` variable as Json value.
        \* ,_stateJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(state)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_stateModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].state # _TETrace[s-1].state
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE AiomeContextEngine_TETrace ----
\*EXTENDS IOUtils, TLC, AiomeContextEngine
\*
\*trace == IODeserialize("AiomeContextEngine_TTrace_1772960605.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE AiomeContextEngine_TETrace ----
EXTENDS TLC, AiomeContextEngine

trace == 
    <<
    ([compacting |-> FALSE,activeSessions |-> {},state |-> "Uninitialized"]),
    ([compacting |-> FALSE,activeSessions |-> {},state |-> "Disposed"])
    >>
----


=============================================================================

---- CONFIG AiomeContextEngine_TTrace_1772960605 ----
CONSTANTS
    Sessions = { "s1" , "s2" }

INVARIANT
    _inv

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Sun Mar 08 18:03:25 JST 2026