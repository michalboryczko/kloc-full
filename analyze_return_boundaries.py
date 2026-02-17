#!/usr/bin/env python3
"""
Analyze Value(local) nodes assigned from method call results
to identify cross-method return boundary cases.
"""

import json
from collections import defaultdict
from typing import Dict, List, Set, Tuple

def load_sot(path: str) -> Dict:
    """Load the sot.json file."""
    with open(path, 'r') as f:
        return json.load(f)

def find_method_call_assignments(sot: Dict) -> List[Dict]:
    """
    Find Value nodes that are assigned from method call results.
    Returns list of (value_node, call_edge, callee_method) tuples.
    """
    results = []
    nodes = {n['id']: n for n in sot['nodes']}
    edges = sot['edges']

    # Build edge index by source
    edges_by_source = defaultdict(list)
    for edge in edges:
        edges_by_source[edge['source']].append(edge)

    # Build edge index by target
    edges_by_target = defaultdict(list)
    for edge in edges:
        edges_by_target[edge['target']].append(edge)

    # Find Value nodes (any kind)
    for node in sot['nodes']:
        if node.get('kind') == 'Value':
            # Look for incoming assigned_from edges from calls
            for edge in edges_by_target.get(node['id'], []):
                if edge.get('type') == 'assigned_from':
                    source_node = nodes.get(edge['source'])
                    if source_node and source_node.get('kind') == 'Call':
                        # Find the callee of this call
                        callee = None
                        for call_edge in edges_by_source.get(source_node['id'], []):
                            if call_edge.get('type') == 'calls':
                                callee = nodes.get(call_edge['target'])
                                break

                        if callee:
                            results.append({
                                'value_node': node,
                                'call_node': source_node,
                                'call_edge': edge,
                                'callee': callee
                            })

    return results

def get_callers(sot: Dict, method_id: str) -> List[Dict]:
    """Get all Call nodes that call this method."""
    callers = []
    for edge in sot['edges']:
        if edge.get('type') == 'calls' and edge['target'] == method_id:
            # Find the call node
            call_node = next((n for n in sot['nodes'] if n['id'] == edge['source']), None)
            if call_node:
                callers.append(call_node)
    return callers

def get_return_info(sot: Dict, method_id: str) -> Dict:
    """
    Analyze what the method returns.
    Returns dict with:
    - has_value_result: whether there's a Value(result) node
    - result_node: the Value(result) node if exists
    - return_edges: produces edges from return expressions
    - assigned_locals: Value(local) nodes that the result is assigned from
    """
    nodes = {n['id']: n for n in sot['nodes']}
    edges_by_source = defaultdict(list)
    edges_by_target = defaultdict(list)
    for edge in sot['edges']:
        edges_by_source[edge['source']].append(edge)
        edges_by_target[edge['target']].append(edge)

    info = {
        'has_value_result': False,
        'result_node': None,
        'return_edges': [],
        'assigned_locals': [],
        'produces_edges': []
    }

    # Find Value(result) nodes via contains edge from method
    for edge in edges_by_source.get(method_id, []):
        if edge.get('type') == 'contains':
            target = nodes.get(edge['target'])
            if target and target.get('kind') == 'Value' and target.get('name') == 'return':
                info['has_value_result'] = True
                info['result_node'] = target

                # Find what produces this result
                for prod_edge in edges_by_target.get(target['id'], []):
                    if prod_edge.get('type') == 'produces':
                        info['produces_edges'].append({
                            'edge': prod_edge,
                            'producer': nodes.get(prod_edge['source'])
                        })
                    elif prod_edge.get('type') == 'assigned_from':
                        source = nodes.get(prod_edge['source'])
                        if source and source.get('kind') == 'Value':
                            info['assigned_locals'].append(source)

                break

    return info

def get_consumers(sot: Dict, value_id: str) -> List[Tuple[str, Dict]]:
    """Get all nodes that consume this value (via used_by edges)."""
    consumers = []
    nodes = {n['id']: n for n in sot['nodes']}

    for edge in sot['edges']:
        if edge.get('type') == 'used_by' and edge['source'] == value_id:
            consumer = nodes.get(edge['target'])
            if consumer:
                param = edge.get('parameter', 'unknown')
                consumers.append((param, consumer))

    return consumers

def main():
    sot_path = '/Users/michal/dev/ai/kloc/artifacts/kloc-dev/context-final/sot.json'
    sot = load_sot(sot_path)

    print("=" * 80)
    print("ANALYSIS: Value nodes assigned from method call results")
    print("=" * 80)
    print()

    assignments = find_method_call_assignments(sot)
    print(f"Found {len(assignments)} Value nodes assigned from method calls")
    print()

    # Filter to cases where the callee has other callers (cross-method cases)
    cross_method_cases = []
    for assignment in assignments:
        callee_id = assignment['callee']['id']
        callers = get_callers(sot, callee_id)
        if len(callers) > 0:  # Has callers
            assignment['num_callers'] = len(callers)
            cross_method_cases.append(assignment)

    print(f"Found {len(cross_method_cases)} cases where callee has callers")
    print()

    # Group by callee method to avoid duplicates
    cases_by_callee = defaultdict(list)
    for case in cross_method_cases:
        callee_fqn = case['callee'].get('fqn', 'unknown')
        cases_by_callee[callee_fqn].append(case)

    print(f"Unique callee methods: {len(cases_by_callee)}")
    print()

    # Analyze each unique callee
    for idx, (callee_fqn, cases) in enumerate(cases_by_callee.items(), 1):
        print("=" * 80)
        print(f"CASE #{idx}: {callee_fqn}")
        print("=" * 80)

        callee = cases[0]['callee']
        print(f"\nCallee ID: {callee['id']}")
        print(f"Callee FQN: {callee_fqn}")
        print(f"Number of call sites: {len(cases)}")

        # Analyze what the callee returns
        return_info = get_return_info(sot, callee['id'])
        print(f"\nReturn analysis:")
        print(f"  Has Value(result): {return_info['has_value_result']}")

        if return_info['has_value_result']:
            result = return_info['result_node']
            print(f"  Result FQN: {result.get('fqn', 'N/A')}")
            print(f"  Result ID: {result['id']}")

            if return_info['produces_edges']:
                print(f"  Produced by {len(return_info['produces_edges'])} expressions:")
                for prod in return_info['produces_edges']:
                    producer = prod['producer']
                    print(f"    - {producer.get('kind', 'unknown')}: {producer.get('fqn', producer.get('id'))}")

            if return_info['assigned_locals']:
                print(f"  Assigned from locals:")
                for local in return_info['assigned_locals']:
                    print(f"    - {local.get('fqn', local.get('id'))}")

            if not return_info['produces_edges'] and not return_info['assigned_locals']:
                print(f"  WARNING: No incoming edges to Value(result)!")

        # Show call sites
        print(f"\nCall sites ({len(cases)}):")
        for call_idx, case in enumerate(cases, 1):
            value = case['value_node']
            call = case['call_node']
            print(f"\n  Site #{call_idx}:")
            print(f"    Value FQN: {value.get('fqn', 'N/A')}")
            print(f"    Value Name: {value.get('name', 'N/A')}")
            print(f"    Value ID: {value['id']}")
            print(f"    Call ID: {call['id']}")

            # Show consumers
            consumers = get_consumers(sot, value['id'])
            if consumers:
                print(f"    Consumers ({len(consumers)}):")
                for param, consumer in consumers[:3]:  # Show first 3
                    print(f"      - {consumer.get('kind', 'unknown')} (param: {param}): {consumer.get('fqn', consumer.get('id'))}")
                if len(consumers) > 3:
                    print(f"      ... and {len(consumers) - 3} more")
            else:
                print(f"    No consumers found!")

        print()

        # Stop after showing first 12 cases
        if idx >= 12:
            print(f"\n... and {len(cases_by_callee) - 12} more cases")
            break

    # Special searches for known examples
    print("\n" + "=" * 80)
    print("SEARCHING FOR SPECIFIC KNOWN EXAMPLES")
    print("=" * 80)

    known_patterns = [
        ('$savedOrder', 'OrderService::createOrder'),
        ('$processedOrder', 'OrderService::createOrder'),
        ('$order', 'OrderService::getOrder'),
        ('$output', 'OrderController::create'),
        ('$output', 'OrderController::get'),
    ]

    for var_name, method_name in known_patterns:
        print(f"\nSearching for {var_name} in {method_name}:")
        found = False

        for node in sot['nodes']:
            if (node.get('kind') == 'Value' and
                var_name in node.get('fqn', '')):

                # Check if this is in the right method
                # Find parent via contains edge
                for edge in sot['edges']:
                    if edge.get('type') == 'contains' and edge['target'] == node['id']:
                        parent = next((n for n in sot['nodes'] if n['id'] == edge['source']), None)
                        if parent and method_name in parent.get('fqn', ''):
                            print(f"  Found: {node.get('fqn')}")
                            print(f"  ID: {node['id']}")
                            print(f"  Name: {node.get('name')}")
                            print(f"  Parent: {parent.get('fqn')}")

                            # Check if assigned from call
                            for e in sot['edges']:
                                if (e.get('type') == 'assigned_from' and
                                    e['target'] == node['id']):
                                    source = next((n for n in sot['nodes'] if n['id'] == e['source']), None)
                                    if source:
                                        print(f"  Assigned from: {source.get('kind')} - {source.get('fqn', source.get('id'))}")

                            found = True
                            break

        if not found:
            print(f"  Not found")

    # Find void return methods
    print("\n" + "=" * 80)
    print("EDGE CASE: Methods with void/no return")
    print("=" * 80)

    void_count = 0
    for node in sot['nodes']:
        if node.get('kind') == 'Method':
            return_info = get_return_info(sot, node['id'])
            if not return_info['has_value_result']:
                callers = get_callers(sot, node['id'])
                if len(callers) > 0:
                    if void_count < 3:
                        print(f"\n{node.get('fqn', node['id'])}")
                        print(f"  Callers: {len(callers)}")
                    void_count += 1

    print(f"\nTotal void/no-return methods with callers: {void_count}")

if __name__ == '__main__':
    main()
