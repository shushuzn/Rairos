import torch
import torch.nn as nn
import torch.nn.functional as F
import math

class StandardSelfAttention(nn.Module):
    def __init__(self, d, heads=1):
        super().__init__()
        self.d = d
        self.heads = heads
        self.head_dim = d // heads
        self.W_q = nn.Linear(d, d)
        self.W_k = nn.Linear(d, d)
        self.W_v = nn.Linear(d, d)
        self.scale = math.sqrt(self.head_dim)

    def forward(self, x):
        B, N, D = x.shape
        q = self.W_q(x).view(B, N, self.heads, self.head_dim).transpose(1, 2)
        k = self.W_k(x).view(B, N, self.heads, self.head_dim).transpose(1, 2)
        v = self.W_v(x).view(B, N, self.heads, self.head_dim).transpose(1, 2)
        attn = torch.einsum("bhnd,bhkd->bhnk", q, k) / self.scale
        attn = F.softmax(attn, dim=-1)
        out = torch.einsum("bhnk,bhkd->bhnd", attn, v)
        out = out.transpose(1, 2).contiguous().view(B, N, D)
        return out

class GalerkinAttention(nn.Module):
    def __init__(self, d, n_modes=16):
        super().__init__()
        self.d = d
        self.n_modes = n_modes
        self.W_q = nn.Linear(d, d)
        self.W_k = nn.Linear(d, d)
        self.W_v = nn.Linear(d, d)
        self.W_out = nn.Linear(d, d)
        self.alpha = nn.Parameter(torch.ones(1))

    def forward(self, x):
        B, N, D = x.shape
        q = self.W_q(x)
        k = self.W_k(x)
        v = self.W_v(x)
        q_fft = torch.fft.rfft(q, dim=1, norm="ortho")
        k_fft = torch.fft.rfft(k, dim=1, norm="ortho")
        n_freqs = q_fft.shape[1]
        M = min(self.n_modes, n_freqs)
        q_m = q_fft[:, :M, :]
        k_m = k_fft[:, :M, :]
        kernel = torch.einsum("bmd,bmd->bmd", q_m, k_m.conj())
        kernel = kernel * self.alpha
        # Project back to full frequency: simple repeat + mirror
        # kernel_full[d] = kernel[d] for d < M, kernel_full[d] = kernel_conj[N-d] for d >= M
        kernel_full = torch.cat([kernel, kernel.conj().flip(1)], dim=1)[:, :n_freqs, :]
        attn = torch.fft.irfft(kernel_full, n=N, dim=1, norm="ortho")
        out = attn * v
        out = self.W_out(out)
        return out

print("=== Attention Mechanism Comparison ===")
d, N, B = 64, 128, 4
x = torch.randn(B, N, d)

attn_std = StandardSelfAttention(d=d)
attn_gal = GalerkinAttention(d=d, n_modes=16)

out_std = attn_std(x)
out_gal = attn_gal(x)

print(f"Input:  {x.shape}")
print(f"Std:    {out_std.shape}, mean={out_std.abs().mean():.4f}")
print(f"Gal:    {out_gal.shape}, mean={out_gal.abs().mean():.4f}")

p_std = sum(p.numel() for p in attn_std.parameters())
p_gal = sum(p.numel() for p in attn_gal.parameters())
print(f"Params - Std: {p_std}, Gal: {p_gal}")

n_test, d_test = 1024, 128
std_ops = n_test**2 * d_test
gal_ops = n_test * d_test**2
print(f"Complexity (n={n_test}, d={d_test}):")
print(f"  Standard: O(n^2d) = {std_ops/1e6:.1f}M ops")
print(f"  Galerkin: O(nd^2) = {gal_ops/1e6:.1f}M ops")
print(f"  Speedup: {std_ops/gal_ops:.1f}x")

print("\n=== Training Comparison ===")
seq = torch.randn(32, N, d)
model_std = StandardSelfAttention(d=d)
model_gal = GalerkinAttention(d=d, n_modes=16)
opt_std = torch.optim.Adam(model_std.parameters(), lr=1e-3)
opt_gal = torch.optim.Adam(model_gal.parameters(), lr=1e-3)
target = torch.randn(32, N, d)

for epoch in range(20):
    out = model_std(seq)
    loss_std = (out - target).pow(2).mean()
    opt_std.zero_grad()
    loss_std.backward()
    opt_std.step()

    out = model_gal(seq)
    loss_gal = (out - target).pow(2).mean()
    opt_gal.zero_grad()
    loss_gal.backward()
    opt_gal.step()

    if (epoch+1) % 5 == 0:
        print(f"  Epoch {epoch+1}: std={loss_std.item():.4f}, gal={loss_gal.item():.4f}")

print("Galerkin attention experiment complete!")
