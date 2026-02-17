#!/usr/bin/env python3
"""
Detailed edge analysis for specific Value nodes.
"""

import json
from collections import defaultdict

def load_sot(path: str):
    with open(path, 'r') as f:
        return json.load(f)

def analyze_value_edges(sot, value_id):
    """Analyze all edges connected to a value node."""
    nodes = {n['id']: n for n in sot['nodes']}

    # Find the value node
    value_node = nodes.get(value_id)
    if not value_node:
        print(f"Value node {value_id} not found")
        return

    print(f"\nValue: {value_node.get('fqn', value_id)}")
    print(f"  Name: {value_node.get('name')}")
    print(f"  Kind: {value_node.get('kind')}")

    # Find all incoming edges
    incoming = [e for e in sot['edges'] if e['target'] == value_id]
    print(f"\nIncoming edges ({len(incoming)}):")
    for edge in incoming:
        source = nodes.get(edge['source'])
        print(f"  {edge['type']:20s} <- {source.get('kind', 'unknown'):15s} {source.get('fqn', source.get('id'))}")

    # Find all outgoing edges
    outgoing = [e for e in sot['edges'] if e['source'] == value_id]
    print(f"\nOutgoing edges ({len(outgoing)}):")
    for edge in outgoing:
        target = nodes.get(edge['target'])
        param = edge.get('parameter', '')
        param_str = f" [{param}]" if param else ""
        print(f"  {edge['type']:20s} -> {target.get('kind', 'unknown'):15s} {target.get('fqn', target.get('id'))}{param_str}")

def main():
    sot_path = '/Users/michal/dev/ai/kloc/artifacts/kloc-dev/context-final/sot.json'
    sot = load_sot(sot_path)

    # Known value IDs from previous output
    known_values = [
        ('node:val:188649a4e0c478cf', '$savedOrder in OrderService::createOrder'),
        ('node:val:a225462f89f038fb', '$processedOrder in OrderService::createOrder'),
        ('node:val:3e7c8dd51d6b9b52', '$order in OrderService::getOrder'),
        ('node:val:5e3aa64a2eb09318', '$output in OrderController::create'),
        ('node:val:d27f0c655c525ac2', '$output in OrderController::get'),
    ]

    print("=" * 80)
    print("EDGE ANALYSIS FOR KNOWN VALUE NODES")
    print("=" * 80)

    for value_id, description in known_values:
        print("\n" + "=" * 80)
        print(description)
        print("=" * 80)
        analyze_value_edges(sot, value_id)

if __name__ == '__main__':
    main()
