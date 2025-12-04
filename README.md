# shapeme-rs

A Rust implementation of image approximation using semi-transparent triangles, inspired by [antirez/shapeme](https://github.com/antirez/shapeme) and [Roger Alsing's genetic algorithm experiment](https://www.rogeralsing.com/2008/12/07/genetic-programming-evolution-of-mona-lisa/).

## Overview

This project approximates images by iteratively placing semi-transparent triangles. Two optimization approaches are provided:

- **Simulated Annealing** (`annealing`) - Mutates a single solution, accepting worse solutions with decreasing probability over time
- **Genetic Algorithm** (`genetic`) - Evolves a population of solutions through selection, crossover, and mutation

## Building

```bash
cargo build --release
```

## Creating Animations

Both binaries save frames to the frames/ directory by default. Use FFmpeg to create a video:

```
# Basic MP4
ffmpeg -framerate 30 -i frames/frame_%06d.png -c:v libx264 -pix_fmt yuv420p evolution.mp4

# Higher quality
ffmpeg -framerate 30 -i frames/frame_%06d.png -c:v libx264 -crf 18 -pix_fmt yuv420p evolution_hq.mp4
```

Or use the included script:

```bash
./animate.sh

# Default (30fps, evolution.mp4, crf=18)
./animate.sh

# Custom framerate
./animate.sh 60

# Custom framerate and output name
./animate.sh 30 mona_lisa.mp4

# Custom framerate, output, and quality (lower crf = higher quality)
./animate.sh 30 mona_lisa_hq.mp4 15
```

## How It Works

# Simulated Annealing

1. Start with a single random triangle
2. Each generation, mutate a random triangle (adjust vertices, color, or alpha)
3. Accept improvements always; accept worse solutions with probability based on temperature
4. Temperature decreases over time (geometric cooling)
5. Periodically add new triangles and briefly "reheat" to allow exploration

# Genetic Algorithm

1. Initialize a population of random individuals (each with a fixed number of triangles)
2. Evaluate fitness as the pixel-wise difference from the target image
3. Select parents via tournament selection
4. Create offspring through single-point crossover
5. Apply random mutations to offspring
6. Preserve the best individuals (elitism)
7. Repeat for many generations

## Output

* SVG - Vector output, scalable to any size
* PNG - Rasterized output at original image dimensions
