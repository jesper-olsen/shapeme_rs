// Approximate a .png image with triangles - learned through genetic optimisation.

// Write final result to .png and .svg files.

// # Basic usage
// cargo run --release --bin shapeme-ga -- image.png
// 
// # More triangles and larger population
// cargo run --release --bin shapeme-ga -- image.png -s 100 -p 100
// 
// # Longer run with higher mutation rate
// cargo run --release --bin shapeme-ga -- image.png -g 50000 -m 0.1
// 
// # Larger tournament size (more selection pressure)
// cargo run --release --bin shapeme-ga -- image.png -k 5
// 
// # More elitism (preserve more top individuals)
// cargo run --release --bin shapeme-ga -- image.png -e 5
// 
// # Custom output
// cargo run --release --bin shapeme-ga -- image.png -o result.svg --output-png result.png
// 
// # Quiet mode, no frames
// cargo run --release --bin shapeme-ga -- image.png -q --frame-interval 0

use clap::Parser;
use image::GenericImageView;
use mersenne_twister_rs::MersenneTwister64;
use rand_core::RngCore;
use shapeme_rs::{save_svg, FrameBuffer, Triangle};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "shapeme-ga")]
#[command(author, version, about = "Approximate images using triangles via genetic algorithm")]
struct Args {
    /// Input image path
    input: String,

    /// Output SVG path
    #[arg(short, long, default_value = "triangles_ga.svg")]
    output: String,

    /// Output PNG path
    #[arg(long, default_value = "triangles_ga.png")]
    output_png: String,

    /// Number of triangles per individual
    #[arg(short = 's', long, default_value_t = 50)]
    num_shapes: usize,

    /// Population size
    #[arg(short, long, default_value_t = 50)]
    population: usize,

    /// Number of generations
    #[arg(short, long, default_value_t = 10_000)]
    generations: u64,

    /// Mutation rate (0.0 - 1.0)
    #[arg(short, long, default_value_t = 0.05)]
    mutation_rate: f64,

    /// Tournament size for selection
    #[arg(short = 'k', long, default_value_t = 3)]
    tournament_size: usize,

    /// Number of elite individuals to preserve
    #[arg(short, long, default_value_t = 2)]
    elitism: usize,

    /// Random seed
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Directory for animation frames (empty to disable)
    #[arg(long, default_value = "frames")]
    frames_dir: String,

    /// Generations between saving frames (0 to disable)
    #[arg(long, default_value_t = 100)]
    frame_interval: u64,

    /// Generations between log output
    #[arg(long, default_value_t = 100)]
    log_interval: u64,

    /// Quiet mode - suppress progress output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

#[derive(Clone)]
struct Individual {
    triangles: Vec<Triangle>,
    fitness: i64,
}

impl Individual {
    fn new<R: RngCore>(rng: &mut R, num_triangles: usize, width: u16, height: u16) -> Self {
        let triangles: Vec<Triangle> = (0..num_triangles)
            .map(|_| Triangle::random(rng, width, height))
            .collect();
        Self {
            triangles,
            fitness: i64::MAX,
        }
    }

    fn evaluate(&mut self, fb: &mut FrameBuffer, reference: &FrameBuffer) {
        fb.clear();
        fb.draw_triangles(&self.triangles);
        self.fitness = fb.diff(reference);
    }

    fn mutate<R: RngCore>(&mut self, rng: &mut R, width: u16, height: u16, mutation_rate: f64) {
        for triangle in &mut self.triangles {
            if (rng.next_u64() as f64 / u64::MAX as f64) < mutation_rate {
                triangle.mutate(rng, width, height);
            }
        }
    }
}

fn crossover<R: RngCore>(parent1: &Individual, parent2: &Individual, rng: &mut R) -> Individual {
    let len = parent1.triangles.len();
    let crossover_point = (rng.next_u64() % len as u64) as usize;

    let mut child_triangles = Vec::with_capacity(len);
    child_triangles.extend_from_slice(&parent1.triangles[..crossover_point]);
    child_triangles.extend_from_slice(&parent2.triangles[crossover_point..]);

    Individual {
        triangles: child_triangles,
        fitness: i64::MAX,
    }
}

fn tournament_select<'a, R: RngCore>(
    population: &'a [Individual],
    rng: &mut R,
    tournament_size: usize,
) -> &'a Individual {
    let mut best: Option<&Individual> = None;

    for _ in 0..tournament_size {
        let idx = (rng.next_u64() % population.len() as u64) as usize;
        let candidate = &population[idx];
        if best.is_none() || candidate.fitness < best.unwrap().fitness {
            best = Some(candidate);
        }
    }

    best.unwrap()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate arguments
    if args.elitism >= args.population {
        eprintln!("Error: elitism must be less than population size");
        std::process::exit(1);
    }
    if args.tournament_size > args.population {
        eprintln!("Error: tournament size must not exceed population size");
        std::process::exit(1);
    }

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
            "Settings: shapes={}, population={}, generations={}, mutation_rate={}",
            args.num_shapes, args.population, args.generations, args.mutation_rate
        );
    }

    let mut rng = MersenneTwister64::new(args.seed);

    let reference = FrameBuffer::from_image(&img);
    let mut fb = FrameBuffer::new(width, height);

    // Initialize population
    let mut population: Vec<Individual> = (0..args.population)
        .map(|_| Individual::new(&mut rng, args.num_shapes, width, height))
        .collect();

    // Evaluate initial population
    for individual in &mut population {
        individual.evaluate(&mut fb, &reference);
    }

    population.sort_by_key(|ind| ind.fitness);

    let mut best_ever = population[0].clone();

    if !args.quiet {
        println!("Initial best fitness: {}", best_ever.fitness);
    }

    for generation in 0..args.generations {
        let mut new_population: Vec<Individual> = Vec::with_capacity(args.population);

        // Elitism
        for i in 0..args.elitism {
            new_population.push(population[i].clone());
        }

        // Generate rest through selection, crossover, mutation
        while new_population.len() < args.population {
            let parent1 = tournament_select(&population, &mut rng, args.tournament_size);
            let parent2 = tournament_select(&population, &mut rng, args.tournament_size);

            let mut child = crossover(parent1, parent2, &mut rng);
            child.mutate(&mut rng, width, height, args.mutation_rate);
            child.evaluate(&mut fb, &reference);

            new_population.push(child);
        }

        population = new_population;
        population.sort_by_key(|ind| ind.fitness);

        if population[0].fitness < best_ever.fitness {
            best_ever = population[0].clone();
        }

        // Logging
        if !args.quiet && args.log_interval > 0 && generation % args.log_interval == 0 {
            println!(
                "Gen {}: best_ever={}, gen_best={}, gen_worst={}",
                generation,
                best_ever.fitness,
                population[0].fitness,
                population[args.population - 1].fitness
            );
        }

        // Save frames
        if !args.frames_dir.is_empty()
            && args.frame_interval > 0
            && generation % args.frame_interval == 0
        {
            fb.clear();
            fb.draw_triangles(&best_ever.triangles);
            let name = format!(
                "{}/frame_{:06}.png",
                args.frames_dir,
                generation / args.frame_interval
            );
            fb.save_png(&name)?;
        }
    }

    if !args.quiet {
        println!("Final best fitness: {}", best_ever.fitness);
        println!("Saving SVG to: {}", args.output);
        println!("Saving PNG to: {}", args.output_png);
    }

    save_svg(&args.output, &best_ever.triangles, width, height)?;

    fb.clear();
    fb.draw_triangles(&best_ever.triangles);
    fb.save_png(&args.output_png)?;

    Ok(())
}
