#!/usr/bin/env python3
"""
Final comprehensive analysis of cross-method return boundaries.
"""

import json
from collections import defaultdict

def load_sot(path: str):
    with open(path, 'r') as f:
        return json.load(f)

def build_indexes(sot):
    nodes = {n['id']: n for n in sot['nodes']}
    edges_by_source = defaultdict(list)
    edges_by_target = defaultdict(list)

    for edge in sot['edges']:
        edges_by_source[edge['source']].append(edge)
        edges_by_target[edge['target']].append(edge)

    return nodes, edges_by_source, edges_by_target

def get_consumers(value_id, edges_by_target, nodes):
    """
    Get consumers of a value. Consumers are Call nodes that have edges
    WITH the value as target and types: receiver, argument.
    The Call is the source of these edges, the Value is the target.
    """
    consumers = []
    for edge in edges_by_target.get(value_id, []):
        if edge.get('type') in ['receiver', 'argument']:
            source = nodes.get(edge['source'])
            if source and source.get('kind') == 'Call':
                consumers.append((edge.get('type'), source, edge))
    return consumers

def main():
    sot_path = '/Users/michal/dev/ai/kloc/artifacts/kloc-dev/context-final/sot.json'
    sot = load_sot(sot_path)

    nodes, edges_by_source, edges_by_target = build_indexes(sot)

    print("=" * 80)
    print("COMPREHENSIVE CROSS-METHOD RETURN BOUNDARY ANALYSIS")
    print("=" * 80)
    print()

    # Find all Value nodes with name starting with $
    local_values = [n for n in sot['nodes']
                   if n.get('kind') == 'Value'
                   and n.get('name', '').startswith('$')]

    print(f"Total local value nodes: {len(local_values)}")

    meaningful_cases = []

    for local in local_values:
        # Check if has consumers (receiver or argument edges pointing to it)
        consumers = get_consumers(local['id'], edges_by_target, nodes)
        if not consumers:
            continue

        # Check if assigned from a call result
        assigned_from_edges = [e for e in edges_by_source.get(local['id'], [])
                               if e.get('type') == 'assigned_from']
        if not assigned_from_edges:
            continue

        for af_edge in assigned_from_edges:
            result = nodes.get(af_edge['target'])
            if not result or result.get('kind') != 'Value':
                continue
            if '(result)' not in result.get('fqn', ''):
                continue

            # Find the call that produced this result
            produces_edges = [e for e in edges_by_target.get(result['id'], [])
                             if e.get('type') == 'produces']
            if not produces_edges:
                continue

            call = nodes.get(produces_edges[0]['source'])
            if not call or call.get('kind') != 'Call':
                continue

            # Find the callee
            calls_edges = [e for e in edges_by_source.get(call['id'], [])
                          if e.get('type') == 'calls']
            if not calls_edges:
                continue

            callee = nodes.get(calls_edges[0]['target'])
            if not callee or callee.get('kind') != 'Method':
                continue

            # Find the return value in the callee
            return_value = None
            for ce in edges_by_source.get(callee['id'], []):
                if ce.get('type') == 'contains':
                    target = nodes.get(ce['target'])
                    if target and target.get('kind') == 'Value' and target.get('name') == 'return':
                        return_value = target
                        break

            if not return_value:
                continue

            # Find what produces the return value
            return_producers = []
            for e in edges_by_target.get(return_value['id'], []):
                if e.get('type') == 'produces':
                    producer = nodes.get(e['source'])
                    if producer:
                        return_producers.append(producer)

            meaningful_cases.append({
                'local': local,
                'result': result,
                'call': call,
                'callee': callee,
                'return_value': return_value,
                'return_producers': return_producers,
                'consumers': consumers,
            })

    print(f"Found {len(meaningful_cases)} meaningful cases")
    print()

    # Group by callee
    by_callee = defaultdict(list)
    for case in meaningful_cases:
        by_callee[case['callee'].get('fqn')].append(case)

    print(f"Across {len(by_callee)} unique callee methods")
    print()

    # Show detailed cases
    for idx, (callee_fqn, cases) in enumerate(sorted(by_callee.items()), 1):
        print("=" * 80)
        print(f"CASE #{idx}: {callee_fqn}")
        print("=" * 80)
        print()

        callee = cases[0]['callee']
        return_value = cases[0]['return_value']
        return_producers = cases[0]['return_producers']

        print(f"Callee Method: {callee_fqn}")
        print(f"  File: {callee.get('file')}")
        print(f"  Line: {callee.get('range', {}).get('start_line')}")
        print()

        print(f"Return Value: {return_value.get('fqn')}")
        print(f"  ID: {return_value['id']}")
        print()

        if return_producers:
            print(f"Return produced by ({len(return_producers)}):")
            for producer in return_producers:
                print(f"  - {producer.get('kind'):15s}: {producer.get('fqn', producer.get('id'))}")
                if producer.get('kind') == 'Value':
                    print(f"    Name: {producer.get('name')}")
                    print(f"    Pattern: Local variable assigned, then returned")
        else:
            print(f"  WARNING: No produces edges!")
        print()

        print(f"Call Sites ({len(cases)}):")
        for site_idx, case in enumerate(cases, 1):
            local = case['local']
            call = case['call']
            consumers = case['consumers']

            print(f"\n  Site #{site_idx}:")
            print(f"    Local Variable: {local.get('fqn')}")
            print(f"      Name: {local.get('name')}")
            print(f"      ID: {local['id']}")
            print(f"    Call: {call.get('fqn')}")
            print(f"      ID: {call['id']}")
            print(f"    Consumers ({len(consumers)}):")
            for edge_type, consumer, edge in consumers[:5]:
                print(f"      - [{edge_type:10s}] {consumer.get('kind'):15s}: {consumer.get('fqn', consumer.get('id'))}")
            if len(consumers) > 5:
                print(f"      ... and {len(consumers) - 5} more")

        print()

        if idx >= 15:
            print(f"\n... and {len(by_callee) - 15} more callee methods")
            break

    # Pattern analysis
    print("\n" + "=" * 80)
    print("PATTERN ANALYSIS")
    print("=" * 80)
    print()

    inline_patterns = []
    local_patterns = []
    mixed_patterns = []

    for callee_fqn, cases in by_callee.items():
        return_producers = cases[0]['return_producers']
        if return_producers:
            producer_kinds = [p.get('kind') for p in return_producers]

            if all(k == 'Value' for k in producer_kinds):
                local_patterns.append(callee_fqn)
            elif any(k == 'Value' for k in producer_kinds):
                mixed_patterns.append(callee_fqn)
            else:
                inline_patterns.append(callee_fqn)

    print(f"Total: {len(by_callee)} callee methods")
    print()
    print(f"INLINE RETURNS ({len(inline_patterns)}): return expr directly")
    for method in inline_patterns[:5]:
        print(f"  - {method}")
    if len(inline_patterns) > 5:
        print(f"  ... and {len(inline_patterns) - 5} more")
    print()

    print(f"LOCAL RETURNS ({len(local_patterns)}): $x = ...; return $x")
    for method in local_patterns[:5]:
        print(f"  - {method}")
    if len(local_patterns) > 5:
        print(f"  ... and {len(local_patterns) - 5} more")
    print()

    print(f"MIXED RETURNS ({len(mixed_patterns)}): multiple return points")
    for method in mixed_patterns[:5]:
        print(f"  - {method}")
    if len(mixed_patterns) > 5:
        print(f"  ... and {len(mixed_patterns) - 5} more")

    # Find specific examples from PHP source
    print("\n" + "=" * 80)
    print("KNOWN EXAMPLE VERIFICATION")
    print("=" * 80)
    print()

    known_examples = {
        'App\\Service\\OrderService::createOrder()': ['$savedOrder', '$processedOrder'],
        'App\\Service\\OrderService::getOrder()': ['$order'],
        'App\\Ui\\Rest\\Controller\\OrderController::create()': ['$output'],
        'App\\Ui\\Rest\\Controller\\OrderController::get()': ['$output'],
    }

    for method_fqn, expected_vars in known_examples.items():
        if method_fqn in by_callee:
            print(f"\n{method_fqn}:")
            cases = by_callee[method_fqn]
            found_vars = [c['local'].get('name') for c in cases]
            print(f"  Expected: {expected_vars}")
            print(f"  Found: {found_vars}")
            print(f"  Status: {'✓ MATCH' if set(found_vars) == set(expected_vars) else '✗ MISMATCH'}")
        else:
            print(f"\n{method_fqn}: NOT FOUND")

if __name__ == '__main__':
    main()
