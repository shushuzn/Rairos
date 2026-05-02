"""

Render gap-feedback-loop animation to professional MP4.

PIL + imageio-ffmpeg.  Deterministic (fixed seed = identical output).

"""

import math, os, numpy as np

from pathlib import Path

import imageio

import imageio_ffmpeg as imff

from PIL import Image, ImageDraw, ImageFont




# ── Config ──────────────────────────────────────────────────────────────────



OUT_DIR   = Path(__file__).parent

VIDEO_OUT = OUT_DIR / "gap-feedback-loop.mp4"

N_FRAMES  = 540      # 18 sec × 30 fps

FPS       = 30

W, H      = 1920, 1088  # 1088 divisible by 16 (h264 macro block)

TITLE_H   = 80

BOTTOM_H  = 220         # height of bottom gene card area

SEED      = 42




# ── Fonts ───────────────────────────────────────────────────────────────────



FONT_DIR = Path(os.path.expanduser("~")) / "AppData" / "Local" / "Microsoft" / "Windows" / "Fonts"

UBUNTU_MONO = FONT_DIR / "UbuntuMono[wght].ttf"



def load_font(size, bold=False):
    try:
        if bold:
            try:
                return ImageFont.truetype(str(FONT_DIR / "UbuntuMono-B.ttf"), size)
            except:
                pass
        return ImageFont.truetype(str(UBUNTU_MONO), size)
    except Exception:
        return ImageFont.load_default()



FONT_HERO    = load_font(48, bold=True)
FONT_TITLE   = load_font(36, bold=True)
FONT_NODE    = load_font(20, bold=True)
FONT_SUB     = load_font(17)
FONT_GENE    = load_font(19)
FONT_STATUS  = load_font(16)
FONT_TINY    = load_font(14)
FONT_PCT     = load_font(13, bold=True)   # for percentage labels on bars




# ── Palette ─────────────────────────────────────────────────────────────────



BG          = (  8,  10,  18)
BG2         = ( 14,  20,  38)
BG_CARD     = ( 10,  16,  32)
BG_PANEL    = (  6,   9,  18)

TOPIC_COL   = (255, 190,  30)   # amber/gold
ANALYZER_COL= ( 60, 210, 200)   # teal
GAP_COL     = (255,  90,  90)    # red
TRACKER_COL = (170,  80, 255)   # purple
GENE_COL    = ( 40, 220, 240)   # cyan
CAPSULE_COL = ( 80, 255, 160)   # mint

PARTICLE_COL= (160, 180, 220)

TEXT        = (230, 238, 250)
TEXT_DIM    = (100, 130, 170)

EDGE_COL    = ( 35,  55,  85)
EDGE_BRIGHT = (100, 140, 210)




# ── Gene data ────────────────────────────────────────────────────────────────



GENE_NAMES = [
    "MultiHead→LinearAttn",
    "Sparse↑Locality",
    "Loss→Stability",
    "PosEnc→Relative",
    "Query→CrossAttn",
    "KVCache→Prefix",
]




# ── Timing helpers ──────────────────────────────────────────────────────────



KEYWORDS = [
    "attention mechanism",
    "transformer efficiency",
    "diffusion models",
    "RLHF",
    "retrieval augmented",
]



def get_keyword(frame):
    return KEYWORDS[(frame // 90) % len(KEYWORDS)]



def gene_target_score(gene_idx, frame):
    target = 0.30 + (gene_idx * 0.11) % 0.50
    t = min(1.0, frame / 150)
    t = t * t * (3 - 2 * t)
    return t * target



def gene_appear_frame(gene_idx):
    return gene_idx * 20



def get_processing_node(frame):
    return (frame // 45) % 6




# ── Text size ────────────────────────────────────────────────────────────────



def text_size(text, font):
    bb = font.getbbox(text)
    return bb[2] - bb[0], bb[3] - bb[1]




# ── Geometry ─────────────────────────────────────────────────────────────────



N_NODES = 6

NODE_LABELS = [
    "Topic Input",
    "GapAnalyzerV2",
    "Gap Objects",
    "EvolutionTracker",
    "CapsuleGene Pool",
    "find_capsule()",
]

# Icon symbols for each node (displayed above the label)
NODE_SYMBOLS = ["◆", "◇", "●", "◈", "▲", "⬡"]

NODE_COLORS = [TOPIC_COL, ANALYZER_COL, GAP_COL, TRACKER_COL, GENE_COL, ANALYZER_COL]

EDGES = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)]

# Layout: circle centered in upper portion of screen
LOOP_CY = (H - BOTTOM_H) // 2 + 20
LOOP_CX = W // 2
RADIUS  = 320
NODE_R  = 72



def node_pos(i):
    angle = -math.pi / 2 + (2 * math.pi / N_NODES) * i
    return (
        LOOP_CX + RADIUS * math.cos(angle),
        LOOP_CY + RADIUS * math.sin(angle),
    )



POSITIONS = [node_pos(i) for i in range(N_NODES)]



def lerp(a, b, t):
    return a + (b - a) * t




# ── Deterministic RNG ─────────────────────────────────────────────────────────



class RNG:
    def __init__(self, seed):
        self.s = seed

    def random(self):
        self.s = (self.s * 1664525 + 1013904223) & 0xFFFFFFFF
        return (self.s >> 8) / 16777215

    def randrange(self, a, b):
        return a + int(self.random() * (b - a))

    def uniform(self, a, b):
        return a + self.random() * (b - a)



rng = RNG(SEED)




# ── Particle system ─────────────────────────────────────────────────────────



N_PARTICLES = 120
N_TRAILS    = 40



class Particle:
    def __init__(self, rng, edge_idx, t):
        self.edge_idx = edge_idx
        self.t = t
        self.speed = 0.0025 + rng.random() * 0.003
        self.size  = 2.0 + rng.random() * 4.0
        self.alpha = 100 + int(rng.random() * 130)
        self.base_col = PARTICLE_COL

    def advance(self):
        self.t += self.speed
        if self.t > 1:
            self.t -= 1
            self.edge_idx = (self.edge_idx + 1) % len(EDGES)



class TrailParticle:
    def __init__(self, rng, edge_idx, t):
        self.edge_idx = edge_idx
        self.t = t
        self.speed = 0.004 + rng.random() * 0.003
        self.size  = 1.5 + rng.random() * 2.5
        self.alpha = 40 + int(rng.random() * 80)
        self.trail = []

    def advance(self):
        from_n, to_n = EDGES[self.edge_idx]
        x = lerp(POSITIONS[from_n][0], POSITIONS[to_n][0], self.t)
        y = lerp(POSITIONS[from_n][1], POSITIONS[to_n][1], self.t)
        self.trail.append((x, y))
        if len(self.trail) > 12:
            self.trail.pop(0)
        self.t += self.speed
        if self.t > 1:
            self.t -= 1
            self.edge_idx = (self.edge_idx + 1) % len(EDGES)
            self.trail = []



def init_particles(rng):
    ps = []
    for i in range(N_PARTICLES):
        ps.append(Particle(rng, i % len(EDGES), rng.random()))
    return ps



def init_trails(rng):
    ts = []
    for i in range(N_TRAILS):
        ts.append(TrailParticle(rng, i % len(EDGES), rng.random()))
    return ts




# ── Draw helpers ─────────────────────────────────────────────────────────────



def draw_glow_ellipse(draw, cx, cy, rx, ry, color, intensity=0.5):
    for layer in range(6, 0, -1):
        frac = layer / 6.0
        alpha = int(60 * intensity * frac)
        rrx = int(rx + layer * 10)
        rry = int(ry + layer * 7)
        bbox = [cx - rrx, cy - rry, cx + rrx, cy + rry]
        draw.ellipse(bbox, fill=color + (alpha,))



def draw_particle(draw, x, y, size, col, alpha):
    for r in range(int(size) + 7, int(size), -1):
        a = int(alpha * 0.08)
        draw.ellipse([x-r, y-r, x+r, y+r], fill=col + (a,))
    draw.ellipse([x-size, y-size, x+size, y+size], fill=col + (alpha,))



def draw_trail(draw, trail, col, alpha):
    if len(trail) < 2:
        return
    for i, (tx, ty) in enumerate(trail):
        frac = i / len(trail)
        a = int(alpha * frac * 0.5)
        s = max(0.5, frac * 2.5)
        draw.ellipse([tx-s, ty-s, tx+s, ty+s], fill=col + (a,))



def draw_edge_line(draw, p1, p2, col, alpha, width=2, dashed=False):
    if dashed:
        dx, dy = p2[0] - p1[0], p2[1] - p1[1]
        length = math.sqrt(dx*dx + dy*dy)
        if length == 0:
            return
        ux, uy = dx/length, dy/length
        dash_len = 12
        gap_len  = 8
        pos = 0
        while pos < length:
            seg_len = min(dash_len, length - pos)
            sx = p1[0] + ux * pos
            sy = p1[1] + uy * pos
            ex = p1[0] + ux * (pos + seg_len)
            ey = p1[1] + uy * (pos + seg_len)
            draw.line([(sx, sy), (ex, ey)], fill=col + (alpha,), width=width)
            pos += dash_len + gap_len
    else:
        draw.line([p1, p2], fill=col + (alpha,), width=width)



def draw_arrowhead(draw, p1, p2, col, alpha, size=14):
    dx, dy = p2[0] - p1[0], p2[1] - p1[1]
    length = math.sqrt(dx*dx + dy*dy)
    if length > 0:
        ux, uy = dx/length, dy/length
        ax = p2[0] - ux * (NODE_R + 8)
        ay = p2[1] - uy * (NODE_R + 8)
        perp = size
        px, py = -uy * perp, ux * perp
        arrow = [
            (p2[0] - ux*size*1.3 + px, p2[1] - uy*size*1.3 + py),
            (p2[0] - ux*size*1.3 - px, p2[1] - uy*size*1.3 - py),
            (ax, ay),
        ]
        draw.polygon(arrow, fill=col + (alpha,))




# ── Background ────────────────────────────────────────────────────────────────



def draw_background(draw, frame):
    draw.rectangle([0, 0, W, H], fill=BG)

    # Radial gradient
    steps = 7
    for s in range(steps, 0, -1):
        frac = s / steps
        alpha = int(18 * frac)
        r = int(min(W, H) * 0.50 * frac)
        bbox = [LOOP_CX - r, LOOP_CY - r, LOOP_CX + r, LOOP_CY + r]
        draw.ellipse(bbox, fill=BG2 + (alpha,))

    # Grid
    grid_col = EDGE_COL + (18,)
    for x in range(0, W, 55):
        draw.line([(x, 0), (x, H)], fill=grid_col, width=1)
    for y in range(0, H, 55):
        draw.line([(0, y), (W, y)], fill=grid_col, width=1)

    # Scanline
    scan_alpha = 6 + int(4 * math.sin(frame * 0.03))
    for sy in range(0, H, 4):
        draw.line([(0, sy), (W, sy)], fill=(0, 0, 0, scan_alpha), width=1)




# ── Title bar ────────────────────────────────────────────────────────────────



SUBTITLE_TEXT = "EvolutionTracker + GapAnalyzerV2 — CapsuleGene闭环"



def draw_title_bar(draw, frame):
    draw.rectangle([0, 0, W, TITLE_H], fill=(4, 6, 14, 255))
    draw.rectangle([0, TITLE_H - 3, W, TITLE_H], fill=GENE_COL + (70,))

    # Left accent bar
    draw.rectangle([0, 0, 6, TITLE_H], fill=ANALYZER_COL + (240,))

    # Hero title with shadow
    title = "Preference-Aware Research Gap Detection"
    tw, th = text_size(title, FONT_HERO)
    draw.text(((W - tw) // 2 + 2, 12 + 2), title, font=FONT_HERO, fill=(0, 0, 0, 60))
    draw.text(((W - tw) // 2, 12), title, font=FONT_HERO, fill=TOPIC_COL + (245,))

    # Subtitle
    sw, sh = text_size(SUBTITLE_TEXT, FONT_TINY)
    draw.text(((W - sw) // 2, TITLE_H - 22), SUBTITLE_TEXT, font=FONT_TINY, fill=TEXT_DIM + (140,))

    # Keyword badge
    kw = get_keyword(frame)
    kw_text = f"Query: {kw}"
    kw_w, kw_h = text_size(kw_text, FONT_STATUS)
    badge_pad = 8
    bx, by = 20, 22
    draw.rounded_rectangle(
        [bx, by, bx + kw_w + badge_pad*2, by + kw_h + badge_pad],
        radius=6,
        fill=(18, 30, 60, 230),
        outline=GENE_COL + (100,),
        width=1
    )
    draw.text((bx + badge_pad, by + badge_pad - 1), kw_text, font=FONT_STATUS, fill=GENE_COL + (210,))

    # Timer
    elapsed = frame / FPS
    timer = f"{elapsed:.1f}s / 18s"
    tw2, _ = text_size(timer, FONT_STATUS)
    draw.text((W - tw2 - 24, 28), timer, font=FONT_STATUS, fill=TEXT_DIM + (150,))

    # Decorative dots
    for i in range(5):
        dx = 20 + i * 16
        dy = TITLE_H - 18
        alpha_dot = 60 + i * 30
        draw.ellipse([dx, dy, dx+5, dy+5], fill=ANALYZER_COL + (alpha_dot,))




# ── Gene panel ───────────────────────────────────────────────────────────────



def draw_bottom_panel(draw, frame):
    panel_y = H - BOTTOM_H

    # Slide-in from bottom
    slide_t = min(1.0, frame / 80)
    slide_t = slide_t * slide_t * (3 - 2 * slide_t)
    slide_offset = int((1 - slide_t) * BOTTOM_H)

    draw.rectangle([0, panel_y, W, H], fill=BG_PANEL + (245,))
    draw.rectangle([0, panel_y, W, panel_y + 3], fill=ANALYZER_COL + (60,))

    # Section label
    label = "C A P S U L E   G E N E   P O O L"
    lw, lh = text_size(label, FONT_STATUS)
    draw.text((36, panel_y + 14), label, font=FONT_STATUS, fill=GENE_COL + (130,))

    # Divider line
    div_y = panel_y + 38
    draw.line([(30, div_y), (W - 30, div_y)], fill=EDGE_COL + (80,), width=1)
    draw.line([(30, div_y), (200, div_y)], fill=GENE_COL + (100,), width=2)

    # Gene cards
    n_cards = len(GENE_NAMES)
    card_w = (W - 80) // n_cards - 8
    card_h = BOTTOM_H - 55
    card_y_base = panel_y + 48
    card_x_start = 36

    for gi, name in enumerate(GENE_NAMES):
        score = gene_target_score(gi, frame)
        appear = gene_appear_frame(gi)
        if frame < appear:
            continue

        # Staggered slide-in
        card_delay = gi * 5
        card_t = max(0, min(1.0, (frame - appear - card_delay) / 25))
        card_t = card_t * card_t * (3 - 2 * card_t)

        fade_t = min(1.0, (frame - appear) / 30)
        card_alpha = int(250 * fade_t * card_t)

        cx_card = card_x_start + gi * (card_w + 8)
        cy_card = card_y_base + int((1 - card_t) * 30)

        # Card background
        draw.rounded_rectangle(
            [cx_card, cy_card, cx_card + card_w, cy_card + card_h],
            radius=8,
            fill=BG_CARD + (card_alpha,),
            outline=GENE_COL + (30 + int(score * 130 * fade_t),),
            width=1
        )

        # Accent bar color based on score
        if score > 0.5:
            bar_col = (80, 220, 160)
        elif score > 0.3:
            bar_col = (255, 200, 60)
        else:
            bar_col = GAP_COL

        bar_h = max(4, int(card_h * 0.10))
        draw.rounded_rectangle(
            [cx_card, cy_card, cx_card + card_w, cy_card + bar_h],
            radius=8,
            fill=bar_col + (int(200 * fade_t * card_t),)
        )
        draw.rectangle([cx_card, cy_card + bar_h - 3, cx_card + card_w, cy_card + bar_h + 2],
                       fill=bar_col + (int(200 * fade_t * card_t),))

        # Gene name
        nw, nh = text_size(name, FONT_GENE)
        tx = cx_card + (card_w - nw) // 2
        ty = cy_card + bar_h + 8
        draw.text((tx + 1, ty + 1), name, font=FONT_GENE, fill=(0, 0, 0, int(80 * fade_t)))
        draw.text((tx, ty), name, font=FONT_GENE, fill=TOPIC_COL + (int(235 * fade_t),))

        # Progress bar
        bar_x = cx_card + 10
        bar_y = cy_card + card_h - 38
        bar_w_inner = card_w - 20
        bar_h_inner = 9

        draw.rounded_rectangle(
            [bar_x, bar_y, bar_x + bar_w_inner, bar_y + bar_h_inner],
            radius=4,
            fill=(20, 32, 55, int(card_alpha * 0.7))
        )

        fill_w = int(bar_w_inner * min(1.0, score))
        if fill_w > 0:
            draw.rounded_rectangle(
                [bar_x, bar_y, bar_x + fill_w, bar_y + bar_h_inner],
                radius=4,
                fill=bar_col + (int(230 * fade_t * card_t),)
            )

        # Percentage label INSIDE the bar (right-aligned)
        pct = int(score * 100)
        pct_text = f"{pct}%"
        pw, ph = text_size(pct_text, FONT_PCT)
        # Draw white label only if there's enough fill to see it
        if fill_w > pw + 6:
            px_label = bar_x + fill_w - pw - 4
            py_label = bar_y - 1
            draw.text((px_label + 1, py_label + 1), pct_text, font=FONT_PCT, fill=(0, 0, 0, 80))
            draw.text((px_label, py_label), pct_text, font=FONT_PCT, fill=(255, 255, 255, 230))

        # Score text below bar
        score_text = f"{score:.2f}"
        sw, _ = text_size(score_text, FONT_STATUS)
        sx = cx_card + (card_w - sw) // 2
        sy = cy_card + card_h - 22
        draw.text((sx + 1, sy + 1), score_text, font=FONT_STATUS, fill=(0, 0, 0, 60))
        score_col = (80, 220, 160) if score > 0.45 else (255, 200, 60) if score > 0.3 else GAP_COL
        draw.text((sx, sy), score_text, font=FONT_STATUS, fill=score_col + (int(220 * fade_t),))




# ── Node rendering ───────────────────────────────────────────────────────────



def draw_node(draw, i, cx, cy, col, is_active, frame):
    pulse = math.sin(frame * 0.10 + i * 0.8) * 0.08 + 0.92

    if is_active:
        draw_glow_ellipse(draw, cx, cy,
                          int(NODE_R * pulse) + 20,
                          int(NODE_R * pulse) + 15,
                          col, intensity=0.8)

        pulse_r = int(NODE_R * pulse) + 14
        draw.ellipse(
            [cx - pulse_r, cy - pulse_r, cx + pulse_r, cy + pulse_r],
            outline=col + (60,),
            width=2
        )
        pulse_r2 = int(NODE_R * pulse) + 26
        draw.ellipse(
            [cx - pulse_r2, cy - pulse_r2, cx + pulse_r2, cy + pulse_r2],
            outline=col + (25,),
            width=1
        )

    # Node fill
    alpha = 255 if is_active else 160
    if not is_active:
        draw.ellipse(
            [cx - NODE_R, cy - NODE_R, cx + NODE_R, cy + NODE_R],
            fill=(10, 16, 35, 230),
            outline=col + (100,),
            width=2
        )
    else:
        for ring in range(5, 0, -1):
            ring_alpha = alpha // (ring + 2)
            ring_r = NODE_R - ring * 5
            draw.ellipse(
                [cx - ring_r, cy - ring_r, cx + ring_r, cy + ring_r],
                fill=col + (ring_alpha,)
            )
        draw.ellipse(
            [cx - NODE_R, cy - NODE_R, cx + NODE_R, cy + NODE_R],
            fill=col + (alpha,),
            outline=col + (240,),
            width=2
        )

    # Symbol icon above node (drawn as colored shape)
    sym = NODE_SYMBOLS[i]
    sym_y = cy - NODE_R - 22
    sw_s, sh_s = text_size(sym, FONT_NODE)
    sx = cx - sw_s // 2
    draw.text((sx + 1, sym_y + 1), sym, font=FONT_NODE, fill=(0, 0, 0, 60))
    draw.text((sx, sym_y), sym, font=FONT_NODE, fill=col + (220 if is_active else 120,))

    # Label
    label = NODE_LABELS[i]
    if i == 0:
        kw = get_keyword(frame)
        kw_short = kw[:18] + ".." if len(kw) > 18 else kw
        label = f"Topic\n{kw_short}"

    lines = label.split("\n")
    line_h = 22
    total_h = len(lines) * line_h
    start_y = cy - total_h // 2 + 2

    for li, line in enumerate(lines):
        lw, lh = text_size(line, FONT_NODE)
        lx = cx - lw // 2
        ly = start_y + li * line_h
        draw.text((lx + 1, ly + 1), line, font=FONT_NODE, fill=(0, 0, 0, 100))
        col_use = TOPIC_COL + (alpha,) if i == 0 else TEXT + (alpha,)
        draw.text((lx, ly), line, font=FONT_NODE, fill=col_use)

    # Small colored dot indicator
    dot_r = 5
    dot_cx = cx + NODE_R - 14
    dot_cy = cy - NODE_R + 14
    draw.ellipse([dot_cx - dot_r, dot_cy - dot_r, dot_cx + dot_r, dot_cy + dot_r],
                 fill=col + (200,))




# ── Edge rendering ───────────────────────────────────────────────────────────



def draw_edges(draw, active_edges, frame):
    for idx, (a, b) in enumerate(EDGES):
        is_active = (a in active_edges or b in active_edges)
        p1, p2 = POSITIONS[a], POSITIONS[b]

        if is_active:
            col = NODE_COLORS[a]
            alpha = 230
            width = 3
            dash_t = (frame + idx * 7) % 20 / 20.0
            draw_edge_line(draw, p1, p2, col, alpha, width, dashed=True)
        else:
            col = EDGE_COL
            alpha = 90
            width = 1
            draw_edge_line(draw, p1, p2, col, alpha, width, dashed=False)

        draw_arrowhead(draw, p1, p2,
                       col if is_active else EDGE_COL,
                       220 if is_active else 90,
                       size=10)




# ── Particles ─────────────────────────────────────────────────────────────────



def draw_particles(draw, particles, active_edges, frame):
    for p in particles:
        p.advance()
        from_n, to_n = EDGES[p.edge_idx]
        fx = lerp(POSITIONS[from_n][0], POSITIONS[to_n][0], p.t)
        fy = lerp(POSITIONS[from_n][1], POSITIONS[to_n][1], p.t)
        alpha = p.alpha
        size  = p.size
        col   = p.base_col
        if from_n in active_edges:
            alpha = min(255, alpha + 90)
            size  = min(size + 2.5, 12)
            col   = NODE_COLORS[from_n]
        draw_particle(draw, fx, fy, size, col, alpha)



def draw_trails(draw, trails, active_edges):
    for t in trails:
        t.advance()
        from_n, to_n = EDGES[t.edge_idx]
        col = NODE_COLORS[from_n] if from_n in active_edges else PARTICLE_COL
        alpha = t.alpha if from_n in active_edges else t.alpha // 2
        draw_trail(draw, t.trail, col, alpha)



def draw_feedback_burst(draw, frame):
    """Backward-flowing feedback burst — starts at 6 seconds."""
    FEEDBACK_FRAME = int(6 * FPS)   # 6 seconds instead of 10
    if frame < FEEDBACK_FRAME:
        return
    burst_offset = frame - FEEDBACK_FRAME
    if burst_offset > 300:
        return

    backward_edges = [(3, 2), (2, 1), (1, 0), (5, 4), (4, 3)]
    for be_idx, (a, b) in enumerate(backward_edges):
        t_norm = (burst_offset + be_idx * 18) % 150 / 150.0
        if t_norm > 0.85:
            continue
        bx = lerp(POSITIONS[a][0], POSITIONS[b][0], t_norm)
        by = lerp(POSITIONS[a][1], POSITIONS[b][1], t_norm)
        alpha = int(255 * (1 - t_norm * 0.5))
        size = 3 + (1 - t_norm) * 5
        col = [GAP_COL, TRACKER_COL, ANALYZER_COL, GENE_COL, TOPIC_COL][be_idx]
        draw_particle(draw, bx, by, size, col, alpha)

        # Trail burst
        for trail_i in range(3):
            trail_t = max(0, t_norm - trail_i * 0.06)
            tx = lerp(POSITIONS[a][0], POSITIONS[b][0], trail_t)
            ty = lerp(POSITIONS[a][1], POSITIONS[b][1], trail_t)
            ta = int(alpha * 0.5 * (1 - trail_i * 0.3))
            draw.ellipse([tx-size*0.5, ty-size*0.5, tx+size*0.5, ty+size*0.5],
                         fill=col + (ta,))




# ── Processing label ──────────────────────────────────────────────────────────



PROCESSING_LABELS = [
    "Analyzing queries...",
    "Scanning papers...",
    "Identifying gaps...",
    "Tracking evolution...",
    "Storing genes...",
    "Finding capsules...",
]



def draw_processing_label(draw, frame):
    proc_node = get_processing_node(frame)
    cx, cy = POSITIONS[proc_node]

    label_y = cy - RADIUS - 55
    if label_y < TITLE_H + 10:
        label_y = cy + RADIUS + 25

    status = PROCESSING_LABELS[proc_node]
    sw, sh = text_size(status, FONT_STATUS)

    px = LOOP_CX - sw//2 - 16
    py = label_y - 8

    draw.rounded_rectangle(
        [px, py, px + sw + 32, py + sh + 16],
        radius=10,
        fill=(8, 14, 28, 235),
        outline=GENE_COL + (80,),
        width=1
    )

    dot_x = px + 10
    dot_y = py + sh//2 + 8
    dot_pulse = int(150 + 105 * math.sin(frame * 0.15))
    draw.ellipse([dot_x - 4, dot_y - 4, dot_x + 4, dot_y + 4],
                 fill=NODE_COLORS[proc_node] + (dot_pulse,))

    draw.text((px + 20, py + 8), status, font=FONT_STATUS, fill=TEXT + (220,))




# ── Stage indicator ──────────────────────────────────────────────────────────



STAGE_LABELS = [
    "INPUT", "ANALYZE", "IDENTIFY", "EVOLVE", "STORE", "RETRIEVE"
]



def draw_stage_indicator(draw, frame):
    proc_node = get_processing_node(frame)
    stage_y = H - BOTTOM_H + 14

    label = f"Stage {proc_node+1}/6: {STAGE_LABELS[proc_node]}"
    lw, lh = text_size(label, FONT_TINY)

    bx = 36
    by = stage_y
    draw.rounded_rectangle([bx, by, bx + lw + 24, by + lh + 10],
                            radius=6,
                            fill=(12, 20, 40, 200),
                            outline=GENE_COL + (50,),
                            width=1)
    draw.text((bx + 12, by + 5), label, font=FONT_TINY, fill=ANALYZER_COL + (200,))

    # Progress dots
    for d in range(6):
        dx = bx + lw + 32 + d * 14
        dy = by + lh//2 + 5
        active = (d <= proc_node)
        col = GENE_COL if active else EDGE_COL
        alpha = 200 if active else 50
        draw.ellipse([dx - 4, dy - 4, dx + 4, dy + 4],
                     fill=col + (alpha,))




# ── Main render ──────────────────────────────────────────────────────────────



def render():
    print(f"Rendering {N_FRAMES} frames → {VIDEO_OUT}")
    print(f"FFMPEG: {imff.get_ffmpeg_exe()}")
    print(f"Resolution: {W}x{H} @ {FPS}fps")

    writer = imageio.get_writer(
        str(VIDEO_OUT),
        fps=FPS,
        codec='libx264',
        pixelformat='yuv420p',
        quality=8,
    )

    rng = RNG(SEED)
    particles = init_particles(rng)
    trails    = init_trails(rng)

    for frame in range(N_FRAMES):
        img = Image.new('RGBA', (W, H), BG + (255,))
        draw = ImageDraw.Draw(img)

        draw_background(draw, frame)
        draw_title_bar(draw, frame)

        proc_node = get_processing_node(frame)
        active_edges = {proc_node, (proc_node - 1) % N_NODES}

        draw_edges(draw, active_edges, frame)
        draw_trails(draw, trails, active_edges)
        draw_particles(draw, particles, active_edges, frame)
        draw_feedback_burst(draw, frame)

        for i in range(N_NODES):
            draw_node(draw, i, POSITIONS[i][0], POSITIONS[i][1],
                      NODE_COLORS[i], i == proc_node, frame)

        draw_processing_label(draw, frame)
        draw_stage_indicator(draw, frame)
        draw_bottom_panel(draw, frame)

        rgb_arr = np.array(img.convert('RGB'))
        writer.append_data(rgb_arr)

        if frame % 60 == 0:
            print(f"  frame {frame}/{N_FRAMES} ({100*frame//N_FRAMES}%)")

    writer.close()
    sz = VIDEO_OUT.stat().st_size
    print(f"\nDone! → {VIDEO_OUT}")
    print(f"Size: {sz/1024/1024:.1f} MB")



if __name__ == "__main__":
    render()
