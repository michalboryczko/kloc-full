#!/usr/bin/env python3
"""
Find ALL cases where a Value(local) is assigned from a Call result,
and trace the return boundary through the callee method.
"""

import json
from collections import defaultdict
from typing import Dict, List, Tuple

def load_sot(path: str):
    with open(path, 'r') as f:
        return json.load(f)

def build_indexes(sot):
    """Build node and edge indexes."""
    nodes = {n['id']: n for n in sot['nodes']}
    edges_by_source = defaultdict(list)
    edges_by_target = defaultdict(list)

    for edge in sot['edges']:
        edges_by_source[edge['source']].append(edge)
        edges_by_target[edge['target']].append(edge)

    return nodes, edges_by_source, edges_by_target

def find_call_result_assignments(sot, nodes, edges_by_source, edges_by_target):
    """
    Find Value nodes that have outgoing assigned_from edges to Value(result) nodes.
    These represent local variables assigned from call results.
    """
    results = []

    for node in sot['nodes']:
        if node.get('kind') == 'Value':
            # Look for outgoing assigned_from edges
            for edge in edges_by_source.get(node['id'], []):
                if edge.get('type') == 'assigned_from':
                    target = nodes.get(edge['target'])
                    # Check if target is a Value(result) node
                    if target and target.get('kind') == 'Value' and '(result)' in target.get('fqn', ''):
                        results.append({
                            'local_value': node,
                            'result_value': target,
                            'edge': edge
                        })

    return results

def get_call_for_result(sot, result_value_id, nodes, edges_by_source, edges_by_target):
    """
    Find the Call node that produced this result value.
    The result value should have incoming produces edge from a Call.
    """
    for edge in edges_by_target.get(result_value_id, []):
        if edge.get('type') == 'produces':
            producer = nodes.get(edge['source'])
            if producer and producer.get('kind') == 'Call':
                return producer
    return None

def get_callee(sot, call_id, nodes, edges_by_source):
    """Get the method that this call invokes."""
    for edge in edges_by_source.get(call_id, []):
        if edge.get('type') == 'calls':
            return nodes.get(edge['target'])
    return None

def get_return_value(sot, method_id, nodes, edges_by_source):
    """Get the Value(return) node for this method."""
    for edge in edges_by_source.get(method_id, []):
        if edge.get('type') == 'contains':
            target = nodes.get(edge['target'])
            if target and target.get('kind') == 'Value' and target.get('name') == 'return':
                return target
    return None

def analyze_return_production(sot, return_value_id, nodes, edges_by_target):
    """
    Analyze what produces the return value.
    Returns: list of (producer_node, edge) tuples
    """
    producers = []
    for edge in edges_by_target.get(return_value_id, []):
        if edge.get('type') == 'produces':
            producer = nodes.get(edge['source'])
            if producer:
                producers.append((producer, edge))
    return producers

def get_consumers(sot, value_id, nodes, edges_by_source):
    """Get all nodes that use this value (via used_by edges)."""
    consumers = []
    for edge in edges_by_source.get(value_id, []):
        if edge.get('type') == 'used_by':
            consumer = nodes.get(edge['target'])
            if consumer:
                param = edge.get('parameter', 'unknown')
                consumers.append((param, consumer, edge))
    return consumers

def main():
    sot_path = '/Users/michal/dev/ai/kloc/artifacts/kloc-dev/context-final/sot.json'
    sot = load_sot(sot_path)

    nodes, edges_by_source, edges_by_target = build_indexes(sot)

    print("=" * 80)
    print("FINDING VALUE NODES ASSIGNED FROM CALL RESULTS")
    print("=" * 80)
    print()

    assignments = find_call_result_assignments(sot, nodes, edges_by_source, edges_by_target)
    print(f"Found {len(assignments)} Value nodes assigned from call results")
    print()

    # Organize by callee method
    by_callee = defaultdict(list)
    for assignment in assignments:
        result_value = assignment['result_value']

        # Find the call that produced this result
        call = get_call_for_result(sot, result_value['id'], nodes, edges_by_source, edges_by_target)
        if not call:
            continue

        # Find the callee method
        callee = get_callee(sot, call['id'], nodes, edges_by_source)
        if not callee:
            continue

        callee_fqn = callee.get('fqn', 'unknown')
        by_callee[callee_fqn].append({
            **assignment,
            'call': call,
            'callee': callee
        })

    print(f"Grouped into {len(by_callee)} unique callee methods")
    print()

    # Analyze each callee
    case_num = 0
    for callee_fqn, cases in sorted(by_callee.items()):
        case_num += 1

        if case_num > 15:
            print(f"\n... and {len(by_callee) - 15} more callee methods")
            break

        print("=" * 80)
        print(f"CASE #{case_num}: {callee_fqn}")
        print("=" * 80)
        print()

        callee = cases[0]['callee']
        print(f"Callee Method: {callee_fqn}")
        print(f"Callee ID: {callee['id']}")
        print(f"File: {callee.get('file', 'N/A')}")
        print(f"Line: {callee.get('range', {}).get('start_line', 'N/A')}")
        print()

        # Analyze what the callee returns
        return_value = get_return_value(sot, callee['id'], nodes, edges_by_source)
        if return_value:
            print(f"Return Value: {return_value.get('fqn', return_value['id'])}")
            print(f"  ID: {return_value['id']}")

            # What produces the return?
            producers = analyze_return_production(sot, return_value['id'], nodes, edges_by_target)
            if producers:
                print(f"\n  Produced by {len(producers)} expression(s):")
                for producer, edge in producers:
                    print(f"    - {producer.get('kind', 'unknown'):15s}: {producer.get('fqn', producer['id'])}")

                    # If producer is a Value, check if it's a local
                    if producer.get('kind') == 'Value':
                        print(f"      Name: {producer.get('name', 'N/A')}")
                        print(f"      Type: Local assigned then returned")
            else:
                print(f"  WARNING: No produces edges found!")
        else:
            print(f"WARNING: No return value found for this method")

        print()

        # Show call sites
        print(f"Call Sites ({len(cases)}):")
        for idx, case in enumerate(cases, 1):
            local_value = case['local_value']
            call = case['call']
            result_value = case['result_value']

            print(f"\n  Site #{idx}:")
            print(f"    Local Value: {local_value.get('fqn', local_value['id'])}")
            print(f"      Name: {local_value.get('name', 'N/A')}")
            print(f"      ID: {local_value['id']}")

            print(f"    Call: {call.get('fqn', call['id'])}")
            print(f"      ID: {call['id']}")

            print(f"    Result Value: {result_value.get('fqn', result_value['id'])}")
            print(f"      ID: {result_value['id']}")

            # Show consumers of the local value
            consumers = get_consumers(sot, local_value['id'], nodes, edges_by_source)
            if consumers:
                print(f"    Consumers of local ({len(consumers)}):")
                for param, consumer, edge in consumers[:5]:
                    print(f"      - {consumer.get('kind', 'unknown'):15s} [{param:10s}]: {consumer.get('fqn', consumer['id'])}")
                if len(consumers) > 5:
                    print(f"      ... and {len(consumers) - 5} more")
            else:
                print(f"    WARNING: No consumers of local value!")

        print()

    # Summary statistics
    print("\n" + "=" * 80)
    print("SUMMARY STATISTICS")
    print("=" * 80)
    print()

    # Count callees with inline returns vs local returns
    inline_count = 0
    local_count = 0
    no_return_count = 0

    for callee_fqn, cases in by_callee.items():
        callee = cases[0]['callee']
        return_value = get_return_value(sot, callee['id'], nodes, edges_by_source)

        if return_value:
            producers = analyze_return_production(sot, return_value['id'], nodes, edges_by_target)
            if producers:
                # Check if any producer is a Value (local)
                has_local = any(p[0].get('kind') == 'Value' for p in producers)
                if has_local:
                    local_count += 1
                else:
                    inline_count += 1
            else:
                no_return_count += 1
        else:
            no_return_count += 1

    print(f"Total callee methods: {len(by_callee)}")
    print(f"  Inline returns (e.g., return new Foo()): {inline_count}")
    print(f"  Local returns (e.g., $x = ...; return $x): {local_count}")
    print(f"  No return value detected: {no_return_count}")

if __name__ == '__main__':
    main()
