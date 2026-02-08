# Observations

1. **Import statements create USES edges from file-level, not from actual usage location**

2. **Usages report import line instead of actual call location**

3. **SCIP is protobuf binary format - cannot concatenate custom sections to it**

4. **SCIP has no Call symbol role - method references and calls look identical**

# Issues

1. **Context query returns all nested usages/deps instead of only direct ones**

2. **scip-php: Constructor args (self::$nextId++) inherit [instantiation] type from enclosing new Order() call at OrderRepository:30**

3. **scip-php: Method call getName() misclassified as [property_access] due to call_kind=access instead of method at OrderService:43**

4. **scip-php: Named args in send(to: savedOrder->customerEmail) misattributed to send() receiver chain instead of savedOrder at OrderService:48**

# Feature Requests

1. **Split uses edge into uses + calls to distinguish type references from method invocations**

2. **Pack index.scip + calls.json into single index.kloc zip archive from indexer**

3. **Add PHP AST pass to extract call sites and argument bindings alongside scip-php**
