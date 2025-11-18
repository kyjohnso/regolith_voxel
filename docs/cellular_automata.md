# Cellular Automata System

## Overview

The Regolith Voxel game implements a height-based cellular automata (CA) system that simulates realistic material flow behavior. Unlike traditional falling-sand simulations that only consider occupied/empty states, this system uses a **heightmap** to track the elevation of materials at each cell, allowing for more nuanced physics including gradual collapse, pile formation, and erosion.

## Core Concepts

### Grid Structure

The world is represented as a 2D grid (512×512 cells by default) where each cell contains:

1. **Material Type** (`MineralType`): The type of mineral at this location
   - Empty, Iron, Copper, Gold, Silver, Uranium, Diamond, Coal

2. **Density** (0.0-1.0): Concentration of the mineral

3. **Height** (`f32`): Elevation value representing how much material is "piled up" at this cell
   - Range: 0-200 (typically)
   - Empty cells have height = 0.0
   - Filled cells have height based on material density and terrain generation

### Physics Types

Each mineral is classified into one of four physics categories:

```rust
enum PhysicsType {
    Empty,      // Void/air - no material present
    Solid,      // Structural materials that don't flow (Diamond, Uranium)
    Granular,   // Sand-like materials that flow when unsupported (Coal, Iron, Copper)
    Flowing,    // Liquid-like materials that spread easily (Gold, Silver)
}
```

### Heightmap Generation

The heightmap is procedurally generated using multi-scale Perlin noise to create varied terrain:

```rust
// Large-scale features (hills and valleys)
let large_variation = noise([x * 0.02, y * 0.02]) * 60.0

// Fine detail (texture)
let detail = noise([x * 0.08, y * 0.08]) * 40.0

// Base height from material density
let base_height = density * 80.0

// Final height
height = max(base_height + large_variation + detail, 5.0)
```

This creates dramatic height variations (5-200 range) with both large geological features and fine surface detail.

## The Algorithm

### Update Cycle

The CA system runs at 30 updates per second (configurable via `CA_TICK_RATE`). Each update:

1. **Clone State**: Create a copy of the current grid and heightmap (`next_data`, `next_heightmap`)
2. **Process Cells**: Iterate through all cells (top to bottom, left to right)
3. **Flow Decision**: For each cell, evaluate potential movements based on height gradients
4. **Apply Changes**: Update the cloned state
5. **Commit**: Replace the current state with the updated state

### Flow Decision Logic

For each cell containing material (not Empty or Solid):

#### 1. Sample Neighbors

Check the 4 cardinal neighbors (N, E, S, W):

```rust
directions = [(0, -1), (1, 0), (0, 1), (-1, 0)]
```

#### 2. Calculate Height Gradient

For each valid neighbor:

```rust
height_diff = current_height - neighbor_height
```

#### 3. Physics-Based Threshold

Materials only consider moving if the height difference exceeds a threshold:

**Granular Materials** (Coal, Iron, Copper):
```rust
if height_diff > 8.0 && random() < 0.5 {
    add_candidate(neighbor)
}
```
- Requires significant height difference (>8 units)
- 50% probability of considering the move
- Models resistance to flow (angle of repose)

**Flowing Materials** (Gold, Silver):
```rust
if height_diff > 4.0 && random() < 0.7 {
    add_candidate(neighbor)
}
```
- Lower threshold (>4 units)
- Higher probability (70%)
- More fluid behavior

#### 4. Candidate Selection

If multiple neighbors are valid:
```rust
if !candidates.empty() && random() < 0.4 {
    chosen = random_choice(candidates)
    transfer_material(current, chosen)
}
```
- 40% probability of actually moving each tick
- Random selection adds natural variation
- Prevents deterministic patterns

### Height Transfer Math

When material moves from cell `A` to cell `B`:

#### Transfer Amount
```rust
height_transfer = 1.0  // One unit per transfer
```

#### Target Cell Logic

**Case 1: Target is Empty or Nearly Empty**
```rust
if target.material == Empty || target.height < 1.0 {
    // Replace with our material
    target.material = source.material
    target.height = target.height + height_transfer
}
```

**Case 2: Target Already Contains Material**
```rust
else {
    // Just add height (visual accumulation)
    target.height = target.height + height_transfer
}
```

#### Source Cell Update

```rust
source.height = source.height - height_transfer

if source.height < 0.5 {
    // Depleted - remove material
    source.material = Empty
    source.height = 0.0
}
```

### Equilibrium Behavior

The system naturally reaches equilibrium when:

```
∀ neighbors: |height[cell] - height[neighbor]| ≤ threshold
```

At equilibrium:
- Granular materials form piles with slopes ≤8 units per cell
- Flowing materials level out to within 4 units
- No further transfers occur

## Mining Mechanics

### Gradual Excavation

Mining reduces height incrementally:

```rust
on_mine_press() {
    for cell in mining_radius {
        cell.height = max(cell.height - 5.0, 0.0)

        if cell.height ≤ 0.5 {
            cell.material = Empty
            cell.height = 0.0
        }
    }
}
```

**Mining Radius**: 10 cells (21×21 area)
**Mining Rate**: -5.0 height per press
**Mining Speed**: Requires ~15-40 presses to fully excavate (depending on initial height)

### Void Creation

When mining creates a void (height = 0):
1. Surrounding high-height cells detect the gradient
2. Materials begin flowing toward the void
3. Height transfers occur gradually (1.0 per tick)
4. Walls "collapse" into the excavated area
5. Eventually reaches a new equilibrium

## Parameters and Tuning

### Global Constants

| Parameter | Value | Location | Description |
|-----------|-------|----------|-------------|
| `MAP_WIDTH` | 512 | main.rs:10 | Grid width in cells |
| `MAP_HEIGHT` | 512 | main.rs:11 | Grid height in cells |
| `CA_TICK_RATE` | 1/30 sec | main.rs:12 | Update frequency (30 Hz) |

### Physics Thresholds

| Material Type | Height Threshold | Move Probability | Flow Rate |
|--------------|------------------|------------------|-----------|
| Granular | >8.0 units | 50% | Slow |
| Flowing | >4.0 units | 70% | Fast |
| Solid | N/A | 0% | Never |

### Height Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Height Transfer | 1.0 | Units transferred per movement |
| Mining Rate | -5.0 | Height removed per mine action |
| Depletion Threshold | 0.5 | Height below which material disappears |
| Height Range | 5-200 | Min/max initial heights |

### Probability Gates

| Stage | Probability | Purpose |
|-------|-------------|---------|
| Consider Move | 50-70% | Physics-dependent threshold check |
| Execute Move | 40% | Global rate limiter for stability |

## Mathematical Properties

### Convergence

The system exhibits **eventual convergence** to a stable state:

**Proof sketch:**
1. Height transfers are conservative (total height is preserved, minus depletion)
2. Materials only flow "downhill" (height[A] > height[B])
3. Transfer amount is bounded (1.0 unit)
4. Probability < 1.0 ensures gradual convergence
5. Thresholds create stable configurations

Therefore: Energy (total potential height) monotonically decreases → stable equilibrium

### Computational Complexity

- **Time Complexity**: O(W × H) per tick where W=width, H=height
- **Space Complexity**: O(2 × W × H) for double buffering
- **Update Rate**: ~8M cell evaluations/second (512² × 30 Hz)

### Stability Analysis

The system remains stable due to:

1. **Double Buffering**: Read from current, write to next (prevents feedback loops)
2. **Probabilistic Updates**: Not all valid moves execute (prevents synchronization artifacts)
3. **Small Transfer Amounts**: 1.0 unit prevents oscillation
4. **Height Thresholds**: Create natural "angle of repose" for piles

## Example Scenarios

### Scenario 1: Mining a Pit

```
Initial State:
Height: [80, 80, 80, 80, 80]
         [80, 80, 80, 80, 80]

After Mining Center:
Height: [80, 80,  0, 80, 80]
         [80, 80, 80, 80, 80]

Tick 1 (height_diff = 80):
Height: [79, 80,  1, 80, 79]  // Neighbors flow in
         [80, 79,  1, 79, 80]

Tick 2-20 (gradual equalization):
Height: [45, 48, 42, 48, 45]  // Approaching equilibrium
         [48, 45, 48, 45, 48]

Final Equilibrium (height_diff < 8.0):
Height: [44, 44, 44, 44, 44]  // Stable configuration
         [44, 44, 44, 44, 44]
```

### Scenario 2: Cliff Collapse

```
Initial:
Height: [100,  10]  // Sharp cliff (diff = 90)

Tick 1:
Height: [ 99,  11]  // 1 unit flows

Tick 2:
Height: [ 98,  12]

...

Equilibrium (after ~45 ticks):
Height: [ 55,  55]  // Level plateau
```

## Implementation Notes

### Performance Optimizations

1. **Early Exit**: Skip empty and solid cells immediately
2. **Double Buffering**: Prevents mid-tick read conflicts
3. **Bounded Iterations**: 4 neighbors only (not 8 diagonal)
4. **Lazy Rendering**: Texture updates only on change detection

### Future Enhancements

Potential improvements to the system:

- **Momentum**: Track velocity for inertia effects
- **Multi-Material Cells**: Height layers of different materials
- **Erosion**: Materials degrade into finer types over time
- **Compression**: Deep materials increase in density
- **Thermal Effects**: Temperature affects flow thresholds

## References

- **Falling Sand Games**: Powder Toy, Sandsplines
- **Height Field Methods**: Smoothed-Particle Hydrodynamics (SPH)
- **Cellular Automata**: von Neumann neighborhoods, Moore neighborhoods
- **Angle of Repose**: Granular physics literature

---

*Last Updated: 2025-11-18*
*Implementation: src/main.rs:1387-1500*
