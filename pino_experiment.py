"""
PINO vs Data-Only comparison on Burgers equation.
"""
import torch
import torch.nn as nn
import numpy as np
from neuralop import LpLoss
from neuralop.models.fno import TFNO

torch.manual_seed(42)
np.random.seed(42)

def solve_burgers(u0, nu=0.01, dt=0.002, n_steps=500):
    u = u0.clone()
    for _ in range(n_steps):
        u_xx = torch.roll(u, -1) - 2*u + torch.roll(u, 1)
        u_x = (torch.roll(u, -1) - torch.roll(u, 1)) * 0.5
        u = u + dt * (-u * u_x + nu * u_xx)
    return u

def generate_burgers_data(n_samples, resolution=128):
    inputs, outputs = [], []
    for _ in range(n_samples):
        x = torch.linspace(0, 2*np.pi, resolution)
        u0 = torch.zeros(resolution)
        for k in range(1, 5):
            u0 += (1/k) * torch.sin(k*x + 2*np.pi*torch.rand(1))
        u0 = u0 / (u0.abs().max() + 1e-8) * 2.0
        u_final = solve_burgers(u0)
        inputs.append(u0.view(1, 1, resolution))
        outputs.append(u_final.view(1, 1, resolution))
    return torch.stack(inputs), torch.stack(outputs)

print("Generating data...")
X_train, Y_train = generate_burgers_data(n_samples=200, resolution=128)
X_test, Y_test = generate_burgers_data(n_samples=40, resolution=128)
print(f"Train: {X_train.shape}, Test: {X_test.shape}")

model_data = TFNO(n_modes=(16, 1), in_channels=1, out_channels=1, hidden_channels=32, n_layers=4)
model_pino = TFNO(n_modes=(16, 1), in_channels=1, out_channels=1, hidden_channels=32, n_layers=4)

loss_fn = LpLoss(d=2)

def pde_residual(u, nu=0.01, dx=2*np.pi/128):
    u_x = (torch.roll(u, -1, dims=-1) - torch.roll(u, 1, dims=-1)) / (2*dx)
    u_xx = (torch.roll(u, -1, dims=-1) - 2*u + torch.roll(u, 1, dims=-1)) / (dx**2)
    return (-u * u_x + nu * u_xx).abs().mean()

print("\n=== Data-Only ===")
opt = torch.optim.Adam(model_data.parameters(), lr=1e-3)
for epoch in range(30):
    model_data.train()
    total = 0
    for i in range(0, len(X_train), 8):
        pred = model_data(X_train[i:i+8])
        loss = loss_fn(pred, Y_train[i:i+8])
        opt.zero_grad()
        loss.backward()
        opt.step()
        total += loss.item()
    if (epoch+1) % 10 == 0:
        print(f"  Epoch {epoch+1}: {total/(len(X_train)//8):.6f}")

print("\n=== PINO ===")
opt2 = torch.optim.Adam(model_pino.parameters(), lr=1e-3)
for epoch in range(30):
    model_pino.train()
    d_loss, p_loss = 0, 0
    for i in range(0, len(X_train), 8):
        xb, yb = X_train[i:i+8], Y_train[i:i+8]
        pred = model_pino(xb)
        dl = loss_fn(pred, yb)
        pl = pde_residual(pred)
        loss = dl + 0.5 * pl
        opt2.zero_grad()
        loss.backward()
        opt2.step()
        d_loss += dl.item()
        p_loss += pl.item()
    if (epoch+1) % 10 == 0:
        print(f"  Epoch {epoch+1}: data={d_loss/(len(X_train)//8):.6f}, pde={p_loss/(len(X_train)//8):.6f}")

model_data.eval()
model_pino.eval()
with torch.no_grad():
    err_data = loss_fn(model_data(X_test), Y_test).item()
    err_pino = loss_fn(model_pino(X_test), Y_test).item()

print("\n=== Results ===")
print(f"Data-Only: {err_data:.6f}")
print(f"PINO:      {err_pino:.6f}")
print(f"Improve:   {(err_data-err_pino)/err_data*100:.1f}%")
