#!/usr/bin/env python3
"""
Scientific Visualization Helper for Rairos MCP Tools.

Generates publication-quality figures using matplotlib with colorblind-safe
palettes and journal-specific styling.

Usage:
    python viz_helper.py --type bar --data '{"labels": ["A", "B"], "values": [1.0, 2.0]}' --output figure.png
    python viz_helper.py --type line --data '{"x": [1, 2, 3], "y": [1.0, 2.0, 1.5]}' --output figure.png
    python viz_helper.py --type heatmap --data '{"data": [[1, 2], [3, 4]]}' --output figure.png
    python viz_helper.py --type radar --data '{"axes": ["Novelty", "Leverage"], "scores": [8, 7]}' --output figure.png
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
    from matplotlib.colors import LinearSegmentedColormap
except ImportError:
    print("ERROR: matplotlib not installed. Run: pip install matplotlib numpy")
    sys.exit(1)

# Okabe-Ito colorblind-safe palette
OKABE_ITO = [
    '#E69F00',  # Orange
    '#56B4E9',  # Sky Blue
    '#009E73',  # Bluish Green
    '#F0E442',  # Yellow
    '#0072B2',  # Blue
    '#D55E00',  # Vermillion
    '#CC79A7',  # Reddish Purple
    '#000000',  # Black
]

# Perceptually uniform colormaps
SEQUENTIAL_COLORMAPS = {
    'viridis': 'viridis',
    'plasma': 'plasma',
    'cividis': 'cividis',
}

DIVERGING_COLORMAPS = {
    'RdBu': 'RdBu_r',
    'PuOr': 'PuOr_r',
    'BrBG': 'BrBG_r',
}


def apply_publication_style():
    """Apply publication-quality base style settings."""
    plt.rcParams.update({
        'figure.dpi': 100,
        'figure.facecolor': 'white',
        'figure.autolayout': True,
        'axes.facecolor': 'white',
        'axes.edgecolor': '#333333',
        'axes.linewidth': 0.8,
        'axes.grid': True,
        'axes.grid.axis': 'y',
        'axes.spines.top': False,
        'axes.spines.right': False,
        'font.family': 'sans-serif',
        'font.sans-serif': ['Arial', 'Helvetica', 'DejaVu Sans'],
        'font.size': 9,
        'axes.labelsize': 10,
        'xtick.labelsize': 8,
        'ytick.labelsize': 8,
        'legend.fontsize': 8,
        'legend.frameon': False,
        'xtick.direction': 'out',
        'ytick.direction': 'out',
        'image.cmap': 'viridis',
        'pdf.fonttype': 42,  # Embed fonts
        'ps.fonttype': 42,
    })


def plot_bar(data: Dict[str, Any], output: str, title: str = "", horizontal: bool = False,
             color: Optional[str] = None, show_values: bool = True, journal: str = "default") -> bool:
    """Generate a bar chart."""
    apply_publication_style()

    labels = data.get('labels', [])
    values = data.get('values', [])
    errors = data.get('errors', [])

    if not labels or not values:
        print("ERROR: 'labels' and 'values' are required for bar chart")
        return False

    # Figure size based on journal
    if journal == "nature":
        width = 3.5  # 89mm single column
    elif journal == "science":
        width = 3.3  # ~84mm
    elif journal == "cell":
        width = 3.4  # ~86mm
    else:
        width = 4.0

    height = width * 0.75
    fig, ax = plt.subplots(figsize=(width, height))

    bar_colors = [color] * len(values) if color else OKABE_ITO[:len(values)]

    if horizontal:
        bars = ax.barh(labels, values, color=bar_colors, height=0.6, edgecolor='none')
        ax.set_xlabel(data.get('xlabel', 'Value'))
        ax.set_ylabel(data.get('ylabel', ''))
        ax.invert_yaxis()
        if show_values:
            for i, (v, bar) in enumerate(zip(values, bars)):
                ax.text(v + max(values) * 0.01, bar.get_y() + bar.get_height()/2,
                       f'{v:.2f}', va='center', fontsize=7)
    else:
        x = np.arange(len(labels))
        bars = ax.bar(x, values, color=bar_colors, width=0.6, edgecolor='none')
        ax.set_xticks(x)
        ax.set_xticklabels(labels, rotation=45, ha='right')
        ax.set_xlabel(data.get('xlabel', ''))
        ax.set_ylabel(data.get('ylabel', 'Value'))
        if show_values:
            for bar in bars:
                height = bar.get_height()
                ax.text(bar.get_x() + bar.get_width()/2., height + max(values) * 0.01,
                       f'{height:.2f}', ha='center', va='bottom', fontsize=7)

    if errors and len(errors) == len(values):
        ax.errorbar(x if not horizontal else np.arange(len(values)), values, yerr=errors,
                   fmt='none', color='#333333', capsize=2, capthick=0.8, linewidth=0.8)

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=10)

    ax.set_xlim(0, max(values) * 1.15 if not horizontal else max(values) * 1.2)
    ax.grid(axis='y', alpha=0.3)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def plot_line(data: Dict[str, Any], output: str, title: str = "",
              color: Optional[str] = None, journal: str = "default") -> bool:
    """Generate a line plot."""
    apply_publication_style()

    x = data.get('x', [])
    y = data.get('y', [])
    y2 = data.get('y2', None)
    labels = data.get('labels', ['Series 1', 'Series 2'])
    errors = data.get('errors', [])

    if not x or not y:
        print("ERROR: 'x' and 'y' are required for line chart")
        return False

    if journal == "nature":
        width = 3.5
    elif journal == "science":
        width = 3.3
    elif journal == "cell":
        width = 3.4
    else:
        width = 4.5

    height = width * 0.7
    fig, ax = plt.subplots(figsize=(width, height))

    line_colors = [color] if color else OKABE_ITO[:2]
    ax.plot(x, y, marker='o', markersize=4, linewidth=1.5, color=line_colors[0], label=labels[0])

    if y2:
        ax.plot(x, y2, marker='s', markersize=4, linewidth=1.5, color=line_colors[1], label=labels[1])

    if errors and len(errors) == len(y):
        ax.fill_between(x, np.array(y) - np.array(errors), np.array(y) + np.array(errors),
                      alpha=0.2, color=line_colors[0])

    ax.set_xlabel(data.get('xlabel', 'X'), fontsize=10)
    ax.set_ylabel(data.get('ylabel', 'Y'), fontsize=10)
    ax.legend(loc='best', frameon=False)
    ax.grid(alpha=0.3)

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=10)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def plot_heatmap(data: Dict[str, Any], output: str, title: str = "",
                 colormap: str = 'viridis', journal: str = "default") -> bool:
    """Generate a heatmap."""
    apply_publication_style()

    matrix = data.get('data', [])
    row_labels = data.get('row_labels', [f'R{i}' for i in range(len(matrix))])
    col_labels = data.get('col_labels', [f'C{i}' for i in range(len(matrix[0]) if matrix else 0)])

    if not matrix:
        print("ERROR: 'data' matrix is required for heatmap")
        return False

    matrix = np.array(matrix)

    if journal == "nature":
        width = 3.5
    elif journal == "science":
        width = 3.3
    elif journal == "cell":
        width = 3.4
    else:
        width = 4.0

    height = width * 0.8
    fig, ax = plt.subplots(figsize=(width, height))

    vmin = data.get('vmin', matrix.min())
    vmax = data.get('vmax', matrix.max())
    cmap_name = SEQUENTIAL_COLORMAPS.get(colormap, colormap) if colormap in SEQUENTIAL_COLORMAPS else colormap

    im = ax.imshow(matrix, aspect='auto', cmap=cmap_name, vmin=vmin, vmax=vmax)

    ax.set_xticks(np.arange(len(col_labels)))
    ax.set_yticks(np.arange(len(row_labels)))
    ax.set_xticklabels(col_labels, rotation=45, ha='right', fontsize=7)
    ax.set_yticklabels(row_labels, fontsize=7)

    cbar = plt.colorbar(im, ax=ax, shrink=0.8)
    cbar.ax.tick_params(labelsize=7)
    cbar.set_label(data.get('colorbar_label', ''), fontsize=8)

    for i in range(len(row_labels)):
        for j in range(len(col_labels)):
            text = ax.text(j, i, f'{matrix[i, j]:.2f}',
                          ha='center', va='center', fontsize=6, color='white' if matrix[i, j] > (vmax + vmin) / 2 else 'black')

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=10)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def plot_radar(data: Dict[str, Any], output: str, title: str = "",
               journal: str = "default") -> bool:
    """Generate a radar chart."""
    apply_publication_style()

    axes = data.get('axes', [])
    scores = data.get('scores', [])
    max_score = data.get('max_score', 10)

    if not axes or not scores:
        print("ERROR: 'axes' and 'scores' are required for radar chart")
        return False

    if len(axes) != len(scores):
        print("ERROR: 'axes' and 'scores' must have same length")
        return False

    angles = np.linspace(0, 2 * np.pi, len(axes), endpoint=False).tolist()
    scores_plot = scores + [scores[0]]
    angles = angles + [angles[0]]

    if journal == "nature":
        size = 3.5
    elif journal == "science":
        size = 3.3
    elif journal == "cell":
        size = 3.4
    else:
        size = 4.0

    fig, ax = plt.subplots(figsize=(size, size), subplot_kw=dict(projection='polar'))

    ax.plot(angles, scores_plot, 'o-', linewidth=1.5, color=OKABE_ITO[0], markersize=5)
    ax.fill(angles, scores_plot, alpha=0.25, color=OKABE_ITO[0])

    ax.set_xticks(angles[:-1])
    ax.set_xticklabels(axes, fontsize=8)
    ax.set_ylim(0, max_score)
    ax.set_yticks(np.linspace(0, max_score, 5))

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=20)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def plot_box(data: Dict[str, Any], output: str, title: str = "",
             journal: str = "default") -> bool:
    """Generate a box plot."""
    apply_publication_style()

    labels = data.get('labels', [])
    datasets = data.get('datasets', [])

    if not labels or not datasets:
        print("ERROR: 'labels' and 'datasets' are required for box plot")
        return False

    if journal == "nature":
        width = 3.5
    elif journal == "science":
        width = 3.3
    elif journal == "cell":
        width = 3.4
    else:
        width = 4.0

    height = width * 0.75
    fig, ax = plt.subplots(figsize=(width, height))

    bp = ax.boxplot(datasets, labels=labels, patch_artist=True, notch=False)

    for patch, color in zip(bp['boxes'], OKABE_ITO[:len(datasets)]):
        patch.set_facecolor(color)
        patch.set_alpha(0.7)

    for median in bp['medians']:
        median.set_color('#333333')
        median.set_linewidth(1.5)

    ax.set_ylabel(data.get('ylabel', 'Value'), fontsize=10)
    ax.grid(axis='y', alpha=0.3)

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=10)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def plot_scatter(data: Dict[str, Any], output: str, title: str = "",
                color: Optional[str] = None, journal: str = "default") -> bool:
    """Generate a scatter plot."""
    apply_publication_style()

    x = data.get('x', [])
    y = data.get('y', [])
    labels = data.get('labels', [])

    if not x or not y:
        print("ERROR: 'x' and 'y' are required for scatter plot")
        return False

    if journal == "nature":
        width = 3.5
    elif journal == "science":
        width = 3.3
    elif journal == "cell":
        width = 3.4
    else:
        width = 4.0

    height = width * 0.8
    fig, ax = plt.subplots(figsize=(width, height))

    scatter_color = color if color else OKABE_ITO[0]
    ax.scatter(x, y, c=scatter_color, s=30, alpha=0.7, edgecolors='white', linewidths=0.5)

    if labels:
        for i, label in enumerate(labels):
            ax.annotate(label, (x[i], y[i]), fontsize=6, alpha=0.8)

    ax.set_xlabel(data.get('xlabel', 'X'), fontsize=10)
    ax.set_ylabel(data.get('ylabel', 'Y'), fontsize=10)
    ax.grid(alpha=0.3)

    if title:
        ax.set_title(title, fontsize=10, fontweight='bold', pad=10)

    plt.savefig(output, dpi=300, bbox_inches='tight', format=output.split('.')[-1])
    plt.close()
    return True


def main():
    parser = argparse.ArgumentParser(description='Rairos Scientific Visualization Helper')
    parser.add_argument('--type', required=True,
                       choices=['bar', 'line', 'heatmap', 'radar', 'box', 'scatter'],
                       help='Chart type')
    parser.add_argument('--data', required=True,
                       help='JSON data for the chart')
    parser.add_argument('--output', required=True,
                       help='Output file path')
    parser.add_argument('--title', default='',
                       help='Chart title')
    parser.add_argument('--color', default=None,
                       help='Single color for all elements')
    parser.add_argument('--journal', default='default',
                       choices=['default', 'nature', 'science', 'cell'],
                       help='Target journal for figure sizing')
    parser.add_argument('--format', default='png',
                       choices=['png', 'pdf', 'svg'],
                       help='Output format')

    args = parser.parse_args()

    try:
        chart_data = json.loads(args.data)
    except json.JSONDecodeError as e:
        print(f"ERROR: Invalid JSON data: {e}")
        sys.exit(1)

    # Update output format if specified
    if args.format != 'png':
        output = args.output.rsplit('.', 1)[0] + '.' + args.format
    else:
        output = args.output

    # Route to appropriate plot function
    success = False
    if args.type == 'bar':
        success = plot_bar(chart_data, output, args.title, args.color, journal=args.journal)
    elif args.type == 'line':
        success = plot_line(chart_data, output, args.title, args.color, journal=args.journal)
    elif args.type == 'heatmap':
        success = plot_heatmap(chart_data, output, args.title, journal=args.journal)
    elif args.type == 'radar':
        success = plot_radar(chart_data, output, args.title, journal=args.journal)
    elif args.type == 'box':
        success = plot_box(chart_data, output, args.title, journal=args.journal)
    elif args.type == 'scatter':
        success = plot_scatter(chart_data, output, args.title, args.color, journal=args.journal)

    if success:
        print(f"SUCCESS: {output}")
        sys.exit(0)
    else:
        print("ERROR: Failed to generate chart")
        sys.exit(1)


if __name__ == '__main__':
    main()
