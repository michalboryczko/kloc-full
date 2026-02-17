#!/usr/bin/env python3
"""
Find meaningful cases: Value(local) assigned from method call results
where the local has consumers AND the callee has a return value.
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

def main():
    sot_path = '/Users/michal/dev/ai/kloc/artifacts/kloc-dev/context-final/sot.json'
    sot = load_sot(sot_path)

    nodes, edges_by_source, edges_by_target = build_indexes(sot)

    # Find Value nodes with name starting with $ (local variables)
    local_values = [n for n in sot['nodes'] if n.get('kind') == 'Value' and n.get('name', '').startswith('$')]

    print("=" * 80)
    print("FINDING MEANINGFUL CROSS-METHOD RETURN BOUNDARY CASES")
    print("=" * 80)
    print()

    meaningful_cases = []

    for local in local_values:
        # Check if this local has used_by edges (consumers)
        consumers = [e for e in edges_by_source.get(local['id'], []) if e.get('type') == 'used_by']
        if not consumers:
            continue

        # Check if this local has an outgoing assigned_from edge to a result
        result_edges = [e for e in edges_by_source.get(local['id'], []) if e.get('type') == 'assigned_from']
        if not result_edges:
            continue

        for result_edge in result_edges:
            result = nodes.get(result_edge['target'])
            if not result or result.get('kind') != 'Value' or '(result)' not in result.get('fqn', ''):
                continue

            # Find the call that produced this result
            produces_edges = [e for e in edges_by_target.get(result['id'], []) if e.get('type') == 'produces']
            if not produces_edges:
                continue

            call = nodes.get(produces_edges[0]['source'])
            if not call or call.get('kind') != 'Call':
                continue

            # Find the callee
            calls_edges = [e for e in edges_by_source.get(call['id'], []) if e.get('type') == 'calls']
            if not calls_edges:
                continue

            callee = nodes.get(calls_edges[0]['target'])
            if not callee or callee.get('kind') != 'Method':
                continue

            # Find the return value in the callee
            contains_edges = [e for e in edges_by_source.get(callee['id'], []) if e.get('type') == 'contains']
            return_value = None
            for ce in contains_edges:
                target = nodes.get(ce['target'])
                if target and target.get('kind') == 'Value' and target.get('name') == 'return':
                    return_value = target
                    break

            if not return_value:
                continue

            # Find what produces the return value
            return_producers = [e for e in edges_by_target.get(return_value['id'], []) if e.get('type') == 'produces']

            meaningful_cases.append({
                'local': local,
                'result': result,
                'call': call,
                'callee': callee,
                'return_value': return_value,
                'return_producers': [nodes.get(e['source']) for e in return_producers],
                'consumers': [nodes.get(e['target']) for e in consumers],
            })

    print(f"Found {len(meaningful_cases)} meaningful cases")
    print()

    # Group by callee
    by_callee = defaultdict(list)
    for case in meaningful_cases:
        by_callee[case['callee'].get('fqn')].append(case)

    print(f"Across {len(by_callee)} unique callee methods")
    print()

    # Show cases
    for idx, (callee_fqn, cases) in enumerate(sorted(by_callee.items()), 1):
        if idx > 12:
            print(f"\n... and {len(by_callee) - 12} more callee methods")
            break

        print("=" * 80)
        print(f"CASE #{idx}: {callee_fqn}")
        print("=" * 80)
        print()

        callee = cases[0]['callee']
        return_value = cases[0]['return_value']
        return_producers = cases[0]['return_producers']

        print(f"Callee: {callee_fqn}")
        print(f"  File: {callee.get('file')}")
        print(f"  Line: {callee.get('range', {}).get('start_line')}")
        print()

        print(f"Return Value: {return_value.get('fqn')}")
        print(f"  ID: {return_value['id']}")
        print()

        if return_producers:
            print(f"Return produced by:")
            for producer in return_producers:
                if producer:
                    print(f"  - {producer.get('kind'):15s}: {producer.get('fqn', producer.get('id'))}")
                    if producer.get('kind') == 'Value':
                        print(f"    Name: {producer.get('name')} (assigned to local then returned)")
        else:
            print(f"WARNING: No produces edges for return!")
        print()

        print(f"Call Sites ({len(cases)}):")
        for site_idx, case in enumerate(cases, 1):
            local = case['local']
            call = case['call']
            consumers = case['consumers']

            print(f"\n  Site #{site_idx}:")
            print(f"    Local: {local.get('fqn')}")
            print(f"      Name: {local.get('name')}")
            print(f"      ID: {local['id']}")
            print(f"    Call: {call.get('fqn')}")
            print(f"      ID: {call['id']}")
            print(f"    Consumers ({len(consumers)}):")
            for consumer in consumers[:5]:
                if consumer:
                    print(f"      - {consumer.get('kind'):15s}: {consumer.get('fqn', consumer.get('id'))}")
            if len(consumers) > 5:
                print(f"      ... and {len(consumers) - 5} more")

        print()

    # Statistics
    print("\n" + "=" * 80)
    print("PATTERN ANALYSIS")
    print("=" * 80)
    print()

    inline_returns = []
    local_returns = []

    for callee_fqn, cases in by_callee.items():
        return_producers = cases[0]['return_producers']
        if return_producers:
            # Check if any producer is a Value (local variable)
            has_local = any(p.get('kind') == 'Value' for p in return_producers if p)
            if has_local:
                local_returns.append(callee_fqn)
            else:
                inline_returns.append(callee_fqn)

    print(f"Total meaningful cases: {len(by_callee)} callees")
    print()
    print(f"Inline returns (return expr): {len(inline_returns)}")
    if inline_returns[:5]:
        for method in inline_returns[:5]:
            print(f"  - {method}")
    print()
    print(f"Local returns ($x = ...; return $x): {len(local_returns)}")
    if local_returns[:5]:
        for method in local_returns[:5]:
            print(f"  - {method}")

if __name__ == '__main__':
    main()
