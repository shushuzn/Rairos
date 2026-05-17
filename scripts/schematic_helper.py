#!/usr/bin/env python3
"""
Scientific Schematic Helper for Rairos MCP Tools.

Generates basic scientific diagrams using matplotlib.
Does NOT require external API keys.

Supports:
- flowchart: CONSORT/PRISMA-style participant flow
- architecture: neural network / system architecture
- pathway: biological signaling pathways
- block: general block diagrams
- timeline: research timeline

Usage:
    python schematic_helper.py --type flowchart --data '{"stages": [{"name": "Start", "count": 500}, ...]}' --output diagram.png
    python schematic_helper.py --type architecture --data '{"layers": [{"name": "Input", "nodes": 784}, ...]}' --output diagram.png
"""

import argparse
import json
import sys
from typing import Dict, Any, List, Optional
import numpy as np

try:
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    import matplotlib.patches as mpatches
    from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
except ImportError:
    print("ERROR: matplotlib not installed. Run: uv pip install matplotlib --system")
    sys.exit(1)

OKABE_ITO = [
    '#E69F00', '#56B4E9', '#009E73', '#F0E442',
    '#0072B2', '#D55E00', '#CC79A7', '#000000'
]

NODE_COLORS = ['#4477AA', '#EE6677', '#228833', '#CCBB44', '#66CCEE', '#AA3377']


def apply_style():
    plt.rcParams.update({
        'figure.dpi': 150,
        'figure.facecolor': 'white',
        'axes.facecolor': 'white',
        'axes.edgecolor': '#333333',
        'axes.linewidth': 0.8,
        'font.family': 'sans-serif',
        'font.size': 9,
        'axes.labelsize': 10,
        'axes.titlesize': 11,
    })


def draw_flowchart(data: Dict[str, Any], output: str) -> bool:
    """Draw a flowchart (CONSORT/PRISMA style)."""
    apply_style()

    stages = data.get('stages', [])
    if not stages:
        print("ERROR: 'stages' required for flowchart")
        return False

    n_stages = len(stages)
    fig_height = max(6, n_stages * 0.9)
    fig, ax = plt.subplots(figsize=(8, fig_height))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, fig_height)
    ax.axis('off')

    box_width = 3.5
    box_height = 0.6
    center_x = 5

    for i, stage in enumerate(stages):
        y_pos = fig_height - 0.8 - i * 0.85
        name = stage.get('name', f'Stage {i+1}')
        count = stage.get('count', '')
        excluded = stage.get('excluded', False)
        branch = stage.get('branch', False)

        color = '#FFAAAA' if excluded else '#AACCFF'
        edge_color = '#666666'

        box = FancyBboxPatch(
            (center_x - box_width/2, y_pos - box_height/2),
            box_width, box_height,
            boxstyle="round,pad=0.05,rounding_size=0.1",
            facecolor=color, edgecolor=edge_color, linewidth=1
        )
        ax.add_patch(box)

        text = f"{name}"
        if count:
            text += f"\nn={count}"
        ax.text(center_x, y_pos, text, ha='center', va='center',
                fontsize=8, fontweight='bold' if not excluded else 'normal')

        if i > 0:
            prev_y = fig_height - 0.8 - (i-1) * 0.85
            ax.annotate('', xy=(center_x, prev_y - box_height/2),
                       xytext=(center_x, y_pos + box_height/2),
                       arrowprops=dict(arrowstyle='->', color='#333333', lw=1.2))

        if branch:
            branch_text = stage.get('branch_text', '')
            branch_y = y_pos - 0.5
            ax.annotate('', xy=(8.5, branch_y), xytext=(center_x + box_width/2, y_pos),
                       arrowprops=dict(arrowstyle='->', color='#333333', lw=1))
            ax.text(8.7, branch_y, branch_text, fontsize=7, va='center')

    ax.set_title(data.get('title', 'Flowchart'), fontsize=12, fontweight='bold', pad=10)
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    plt.close()
    return True


def draw_architecture(data: Dict[str, Any], output: str) -> bool:
    """Draw a neural network / system architecture diagram."""
    apply_style()

    layers = data.get('layers', [])
    if not layers:
        print("ERROR: 'layers' required for architecture")
        return False

    n_layers = len(layers)
    fig_width = max(8, n_layers * 1.5)
    fig, ax = plt.subplots(figsize=(fig_width, 5))
    ax.set_xlim(-0.5, n_layers + 0.5)
    ax.set_ylim(-1.5, 1.5)
    ax.axis('off')

    layer_colors = [NODE_COLORS[i % len(NODE_COLORS)] for i in range(n_layers)]

    for i, layer in enumerate(layers):
        name = layer.get('name', f'Layer {i}')
        nodes = layer.get('nodes', 1)
        layer_type = layer.get('type', 'dense')
        color = layer.get('color', layer_colors[i])

        x = i
        layer_height = min(nodes * 0.15 + 0.3, 1.2)

        box = FancyBboxPatch(
            (x - 0.35, -layer_height/2),
            0.7, layer_height,
            boxstyle="round,pad=0.02",
            facecolor=color, edgecolor='#333333', linewidth=1.5, alpha=0.8
        )
        ax.add_patch(box)
        ax.text(x, 0, name, ha='center', va='center', fontsize=8, color='white', fontweight='bold')

        label_y = -layer_height/2 - 0.25
        ax.text(x, label_y, f'n={nodes}', ha='center', va='top', fontsize=7, color='#666666')

        if i > 0:
            prev_layer = layers[i-1]
            prev_nodes = prev_layer.get('nodes', 1)
            prev_height = min(prev_nodes * 0.15 + 0.3, 1.2)
            for j in range(min(nodes, prev_nodes, 8)):
                t = (j - min(nodes, prev_nodes)/2 + 0.5) / min(nodes, prev_nodes)
                y1 = -prev_height/2 + t * prev_height
                y2 = -layer_height/2 + t * layer_height
                ax.plot([i-1+0.35, x-0.35], [y1, y2], '-', color='#888888', lw=0.5, alpha=0.4)

    ax.set_title(data.get('title', 'Architecture'), fontsize=12, fontweight='bold', pad=10)
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    plt.close()
    return True


def draw_pathway(data: Dict[str, Any], output: str) -> bool:
    """Draw a biological signaling pathway."""
    apply_style()

    steps = data.get('steps', [])
    if not steps:
        print("ERROR: 'steps' required for pathway")
        return False

    n_steps = len(steps)
    fig_width = max(6, n_steps * 1.2)
    fig, ax = plt.subplots(figsize=(fig_width, 4))
    ax.set_xlim(-0.5, n_steps + 0.5)
    ax.set_ylim(-1, 1)
    ax.axis('off')

    for i, step in enumerate(steps):
        name = step.get('name', f'Step {i+1}')
        arrow_type = step.get('arrow', '->')
        label = step.get('label', '')
        color = step.get('color', OKABE_ITO[i % len(OKABE_ITO)])

        x = i
        circle = plt.Circle((x, 0), 0.25, facecolor=color, edgecolor='#333333', linewidth=1.5)
        ax.add_patch(circle)
        ax.text(x, 0, f'{i+1}', ha='center', va='center', fontsize=8, color='white', fontweight='bold')
        ax.text(x, -0.45, name, ha='center', va='top', fontsize=8, fontweight='bold')
        if label:
            ax.text(x, 0.4, label, ha='center', va='bottom', fontsize=7, color='#666666')

        if i < n_steps - 1:
            arrow_color = '#009E73' if arrow_type == '->' else '#D55E00'
            ax.annotate('', xy=(i+1-0.3, 0), xytext=(i+0.3, 0),
                       arrowprops=dict(arrowstyle=f'{arrow_type}', color=arrow_color, lw=2))

    ax.set_title(data.get('title', 'Signaling Pathway'), fontsize=12, fontweight='bold', pad=10)
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    plt.close()
    return True


def draw_block(data: Dict[str, Any], output: str) -> bool:
    """Draw a general block diagram."""
    apply_style()

    blocks = data.get('blocks', [])
    connections = data.get('connections', [])
    title = data.get('title', 'Block Diagram')

    if not blocks:
        print("ERROR: 'blocks' required for block diagram")
        return False

    n_blocks = len(blocks)
    cols = min(3, n_blocks)
    rows = (n_blocks + cols - 1) // cols
    fig_width = cols * 2.5 + 0.5
    fig_height = rows * 1.5 + 0.5

    fig, ax = plt.subplots(figsize=(fig_width, fig_height))
    ax.set_xlim(-0.2, cols * 2.5 + 0.2)
    ax.set_ylim(-0.2, rows * 1.5 + 0.2)
    ax.axis('off')

    for i, block in enumerate(blocks):
        col = i % cols
        row = i // cols
        x = col * 2.5 + 1.25
        y = rows * 1.5 - row * 1.5 - 0.75

        name = block.get('name', f'Block {i+1}')
        color = block.get('color', '#4477AA')

        box = FancyBboxPatch(
            (x - 0.9, y - 0.4),
            1.8, 0.8,
            boxstyle="round,pad=0.05",
            facecolor=color, edgecolor='#333333', linewidth=1.5, alpha=0.8
        )
        ax.add_patch(box)
        ax.text(x, y, name, ha='center', va='center', fontsize=9, color='white', fontweight='bold')

    for conn in connections:
        from_idx = conn.get('from', 0)
        to_idx = conn.get('to', 0)
        from_col = from_idx % cols
        from_row = from_idx // cols
        to_col = to_idx % cols
        to_row = to_idx // cols

        x1 = from_col * 2.5 + 1.25
        y1 = from_row * 1.5 + rows * 1.5 - 0.75
        x2 = to_col * 2.5 + 1.25 - 0.9
        y2 = to_row * 1.5 + rows * 1.5 - 0.75

        ax.annotate('', xy=(x2, y2), xytext=(x1 + 0.9, y1),
                   arrowprops=dict(arrowstyle='->', color='#333333', lw=1.2))

    ax.set_title(title, fontsize=12, fontweight='bold', pad=10)
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    plt.close()
    return True


def draw_timeline(data: Dict[str, Any], output: str) -> bool:
    """Draw a research timeline."""
    apply_style()

    phases = data.get('phases', [])
    if not phases:
        print("ERROR: 'phases' required for timeline")
        return False

    n_phases = len(phases)
    fig_width = max(8, n_phases * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 3))
    ax.set_xlim(-0.5, n_phases + 0.5)
    ax.set_ylim(-1, 1)
    ax.axis('off')

    ax.axhline(y=0, color='#333333', linewidth=2, zorder=0)

    for i, phase in enumerate(phases):
        name = phase.get('name', f'Phase {i+1}')
        duration = phase.get('duration', '')
        color = phase.get('color', OKABE_ITO[i % len(OKABE_ITO)])

        x = i
        circle = plt.Circle((x, 0), 0.2, facecolor=color, edgecolor='#333333', linewidth=1.5, zorder=2)
        ax.add_patch(circle)
        ax.text(x, 0.4, name, ha='center', va='bottom', fontsize=9, fontweight='bold')
        if duration:
            ax.text(x, -0.35, duration, ha='center', va='top', fontsize=7, color='#666666')

        if i < n_phases - 1:
            ax.plot([i+0.2, i+1-0.2], [0, 0], '-', color='#333333', lw=2, zorder=1)

    ax.set_title(data.get('title', 'Timeline'), fontsize=12, fontweight='bold', pad=10)
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    plt.close()
    return True


def main():
    parser = argparse.ArgumentParser(description='Rairos Scientific Schematic Helper')
    parser.add_argument('--type', required=True,
                       choices=['flowchart', 'architecture', 'pathway', 'block', 'timeline'],
                       help='Diagram type')
    parser.add_argument('--data', required=True,
                       help='JSON data for the diagram')
    parser.add_argument('--output', required=True,
                       help='Output file path')
    parser.add_argument('--title', default='',
                       help='Diagram title')

    args = parser.parse_args()

    try:
        diagram_data = json.loads(args.data)
    except json.JSONDecodeError as e:
        print(f"ERROR: Invalid JSON: {e}")
        sys.exit(1)

    if args.title:
        diagram_data['title'] = args.title

    success = False
    if args.type == 'flowchart':
        success = draw_flowchart(diagram_data, args.output)
    elif args.type == 'architecture':
        success = draw_architecture(diagram_data, args.output)
    elif args.type == 'pathway':
        success = draw_pathway(diagram_data, args.output)
    elif args.type == 'block':
        success = draw_block(diagram_data, args.output)
    elif args.type == 'timeline':
        success = draw_timeline(diagram_data, args.output)

    if success:
        print(f"SUCCESS: {args.output}")
        sys.exit(0)
    else:
        print("ERROR: Failed to generate diagram")
        sys.exit(1)


if __name__ == '__main__':
    main()
