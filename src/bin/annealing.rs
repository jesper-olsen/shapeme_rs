// Simulated annealing - approximate a .png image with triangles.
// Write final result to .png and .svg files and optionally intermediate
// "frames" so that the process can be annimated.

// # Basic usage
// cargo run --release -- Assets/mona_lisa_400x596.png
//
// # Custom output names
// cargo run --release -- Assets/mona_lisa.png -o mona.svg --output-png mona.png
//
// # More triangles, longer run
// cargo run --release -- image.png -s 256 -g 1000000
//
// # Faster cooling (converges quicker but maybe worse result)
// cargo run --release -- image.png -c 0.9999
//
// # No animation frames
// cargo run --release -- image.png --frame-interval 0
//
// # Different seed for reproducibility
// cargo run --release -- image.png --seed 12345
//
// # Quiet mode
// cargo run --release -- image.png -q
//
// # Show help
// cargo run --release -- --help

use clap::Parser;
use image::GenericImageView;
use mersenne_twister_rs::MersenneTwister64;
use rand_core::RngCore;
use shapeme_rs::{FrameBuffer, Triangle, save_svg};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "shapeme")]
#[command(
    author,
    version,
    about = "Approximate images using triangles via simulated annealing"
)]
struct Args {
    /// Input image path
    input: String,

    /// Output SVG path
    #[arg(short, long, default_value = "triangles.svg")]
    output: String,

    /// Output PNG path
    #[arg(long, default_value = "triangles.png")]
    output_png: String,

    /// Maximum number of triangles
    #[arg(short = 's', long, default_value_t = 128)]
    num_shapes: usize,

    /// Number of generations
    #[arg(short, long, default_value_t = 500_000)]
    generations: u64,

    /// Cooling rate for simulated annealing
    #[arg(short, long, default_value_t = 0.99995)]
    cooling_rate: f64,

    /// Initial temperature
    #[arg(short, long, default_value_t = 1.0)]
    temperature: f64,

    /// Generations between adding new triangles
    #[arg(long, default_value_t = 2000)]
    add_interval: u64,

    /// Reheat temperature when adding shapes
    #[arg(long, default_value_t = 0.01)]
    reheat_temp: f64,

    /// Random seed
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Directory for animation frames (empty to disable)
    #[arg(long, default_value = "frames")]
    frames_dir: String,

    /// Generations between saving frames (0 to disable)
    #[arg(long, default_value_t = 200)]
    frame_interval: u64,

    /// Generations between log output
    #[arg(long, default_value_t = 1000)]
    log_interval: u64,

    /// Quiet mode - suppress progress output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Create frames directory if needed
    if !args.frames_dir.is_empty() && args.frame_interval > 0 {
        std::fs::create_dir_all(&args.frames_dir)?;
    }

    let img = image::open(Path::new(&args.input))?;
    let (width, height) = img.dimensions();
    let (width, height) = (width as u16, height as u16);

    if !args.quiet {
        println!("Successfully loaded image: {width}x{height}");
        println!(
            "Settings: num_shapes={}, generations={}, cooling_rate={}",
            args.num_shapes, args.generations, args.cooling_rate
        );
    }

    let mut rng = MersenneTwister64::new(args.seed);
    let mut triangles: Vec<Triangle> = Vec::with_capacity(args.num_shapes);
    triangles.push(Triangle::random(&mut rng, width, height));

    let reference = FrameBuffer::from_image(&img);
    let mut fb = FrameBuffer::new(width, height);

    fb.clear();
    fb.draw_triangles(&triangles);
    let mut current_diff = fb.diff(&reference);
    let mut best_diff = current_diff;
    let mut best_triangles = triangles.clone();

    if !args.quiet {
        println!("Initial diff: {current_diff}");
    }

    let mut temperature = args.temperature;

    for generation in 0..args.generations {
        // Geometric cooling
        temperature *= args.cooling_rate;

        // Add triangles periodically
        if generation % args.add_interval == 0
            && generation > 0
            && triangles.len() < args.num_shapes
        {
            triangles.push(Triangle::random(&mut rng, width, height));
            temperature = temperature.max(args.reheat_temp);
            fb.clear();
            fb.draw_triangles(&triangles);
            current_diff = fb.diff(&reference);
        }

        // === Mutate ===
        let mut triangles_p = triangles.clone();
        let idx = (rng.next_u64() % triangles_p.len() as u64) as usize;
        triangles_p[idx].mutate(&mut rng, width, height);

        fb.clear();
        fb.draw_triangles(&triangles_p);
        let new_diff = fb.diff(&reference);

        // Acceptance decision
        let accept = if new_diff < current_diff {
            true
        } else if temperature > 1e-10 {
            let delta = (new_diff - current_diff) as f64;
            let normalized_delta = delta / (current_diff as f64 + 1.0);
            let p = (-normalized_delta / temperature).exp();
            (rng.next_u64() as f64 / u64::MAX as f64) < p
        } else {
            false
        };

        if accept {
            triangles = triangles_p;
            current_diff = new_diff;

            if current_diff < best_diff {
                best_diff = current_diff;
                best_triangles = triangles.clone();
            }
        }

        // Logging
        if !args.quiet && args.log_interval > 0 && generation % args.log_interval == 0 {
            println!(
                "Gen {generation}/{}: current={current_diff}, best={best_diff}, temp={temperature:.6}, triangles={}",
                triangles.len(),
                args.generations
            );
        }

        // Save frames
        if !args.frames_dir.is_empty()
            && args.frame_interval > 0
            && generation % args.frame_interval == 0
        {
            fb.clear();
            fb.draw_triangles(&best_triangles);
            let name = format!(
                "{}/frame_{:06}.png",
                args.frames_dir,
                generation / args.frame_interval
            );
            fb.save_png(&name)?;
        }
    }

    // Final output
    if !args.quiet {
        println!("Final best diff: {best_diff}");
        println!("Saving SVG to: {}", args.output);
        println!("Saving PNG to: {}", args.output_png);
    }

    save_svg(&args.output, &best_triangles, width, height)?;

    fb.clear();
    fb.draw_triangles(&best_triangles);
    fb.save_png(&args.output_png)?;

    Ok(())
}
