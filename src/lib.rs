use rand_core::RngCore;
use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub mod frame_buffer;
pub use frame_buffer::FrameBuffer;

#[derive(Clone)]
struct Colour {
    r: u8,
    g: u8,
    b: u8,
    alpha: u8,
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rgba({}, {}, {}, {})",
            self.r, self.g, self.b, self.alpha
        )
    }
}

const MINALPHA: u8 = 1;
const MAXALPHA: u8 = 100;

impl Colour {
    pub fn random<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        let bits = rng.next_u64();
        // we only use 4 bytes...
        Self {
            r: bits as u8,
            g: (bits >> 8) as u8,
            b: (bits >> 16) as u8,
            alpha: MINALPHA + ((bits >> 24) as u8 % (MAXALPHA - MINALPHA + 1)),
        }
    }

    pub fn mutate_colour<R: RngCore + ?Sized>(&mut self, rng: &mut R, delta: u8) {
        let idelta = delta as i64;
        self.r = (self.r as i64 + rand_between(rng, -idelta, idelta)).clamp(0, 255) as u8;
        self.g = (self.g as i64 + rand_between(rng, -idelta, idelta)).clamp(0, 255) as u8;
        self.b = (self.b as i64 + rand_between(rng, -idelta, idelta)).clamp(0, 255) as u8;
    }

    pub fn mutate_alpha<R: RngCore + ?Sized>(&mut self, rng: &mut R, delta: u8) {
        let idelta = delta as i64;
        let x = rand_between(rng, -idelta, idelta);
        self.alpha = (self.alpha as i64 + x).clamp(MINALPHA as i64, MAXALPHA as i64) as u8;
    }
}

#[derive(Clone)]
pub struct Triangle {
    vertices: [(u16, u16); 3],
    colour: Colour,
}

impl fmt::Display for Triangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [(x1, y1), (x2, y2), (x3, y3)] = self.vertices;
        write!(
            f,
            "Triangle [({x1},{y1}),({x2},{y2}),({x3},{y3})] {}",
            self.colour
        )
    }
}

fn rand_u16_x4<R: RngCore + ?Sized>(rng: &mut R) -> (u16, u16, u16, u16) {
    let bits = rng.next_u64();
    let a = bits as u16;
    let b = (bits >> 16) as u16;
    let c = (bits >> 32) as u16;
    let d = (bits >> 48) as u16;
    (a, b, c, d)
}

// return random number in the specified range (including min and max)
fn rand_between<R: RngCore + ?Sized>(rng: &mut R, min: i64, max: i64) -> i64 {
    let range = (max - min + 1) as u64;
    let r = rng.next_u64() % range;
    min + r as i64
}

impl Triangle {
    // random colour and random placement inside canvas
    pub fn random<R: RngCore + ?Sized>(rng: &mut R, width: u16, height: u16) -> Self {
        let (a, b, c, d) = rand_u16_x4(rng);
        let (e, f, _, _) = rand_u16_x4(rng);
        let mut t = Triangle {
            colour: Colour::random(rng),
            vertices: [
                (a % width, b % height),
                (c % width, d % height),
                (e % width, f % height),
            ],
        };
        t.normalise(width, height);
        t
    }

    // mutate: randomly move vertices
    fn mutate_vertices<R: RngCore + ?Sized>(
        &mut self,
        rng: &mut R,
        width: u16,
        height: u16,
        delta: u16,
    ) {
        let delta_i = delta as i64;
        for i in 0..3 {
            let dx = rand_between(rng, -delta_i, delta_i);
            let dy = rand_between(rng, -delta_i, delta_i);
            self.vertices[i].0 = (self.vertices[i].0 as i64 + dx).clamp(0, width as i64 - 1) as u16;
            self.vertices[i].1 =
                (self.vertices[i].1 as i64 + dy).clamp(0, height as i64 - 1) as u16;
        }
        self.normalise(width, height);
    }

    //When we mutate a triangle, or create a random one, it is possible that the
    //result is invalid: coordinates out of the screen or the points not ordered
    //by 'y' (that is required for our triangle drawing algorith).
    //
    //This function normalizes it turning an invalid triangle into a valid one. */
    fn normalise(&mut self, width: u16, height: u16) {
        // Sort vertices by Y-coordinate (Ascending) to ensure y1 <= y2 <= y3.
        self.vertices.sort_by(|a, b| a.1.cmp(&b.1));

        // Clamp coordinates to fit inside the canvas
        let max_x = width.saturating_sub(1);
        let max_y = height.saturating_sub(1);
        for vertex in &mut self.vertices {
            if vertex.0 > max_x {
                vertex.0 = max_x;
            }
            if vertex.1 > max_y {
                vertex.1 = max_y;
            }
        }
    }

    // Apply a random mutation
    pub fn mutate<R: RngCore + ?Sized>(&mut self, rng: &mut R, width: u16, height: u16) {
        match rng.next_u64() % 10 {
            // Changed from 6
            0 => *self = Triangle::random(rng, width, height),
            1 | 2 => self.mutate_vertices(rng, width, height, 3), // Small vertex moves
            3 | 4 => self.mutate_vertices(rng, width, height, 10), // Medium vertex moves
            5 | 6 => self.colour.mutate_colour(rng, 10),
            7 | 8 => self.colour.mutate_colour(rng, 30),
            _ => self.colour.mutate_alpha(rng, 10),
        }
    }
}

pub fn save_svg<P: AsRef<Path>>(
    filename: P,
    triangles: &[Triangle],
    width: u16,
    height: u16,
) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut w = BufWriter::new(file);

    // Header
    writeln!(
        w,
        r#"<?xml version="1.0" standalone="no"?><!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd"><svg width="100%" height="100%" style="background-color:#000000;" version="1.1" xmlns="http://www.w3.org/2000/svg">"#
    )?;

    // Black background rectangle
    writeln!(
        w,
        r#"<polygon points="0,0 {},0 {},{} 0,{}" style="fill:#000000;stroke:#000000;stroke-width:0;fill-opacity:1;"/>"#,
        width - 1,
        width - 1,
        height - 1,
        height - 1
    )?;

    // Triangles
    for t in triangles {
        let [(x1, y1), (x2, y2), (x3, y3)] = t.vertices;
        let c = &t.colour;
        let opacity = c.alpha as f32 / 100.0;
        writeln!(
            w,
            r#"<polygon points="{},{} {},{} {},{}" style="fill:#{:02x}{:02x}{:02x};stroke:#000000;stroke-width:0;fill-opacity:{:.2};"/>"#,
            x1, y1, x2, y2, x3, y3, c.r, c.g, c.b, opacity
        )?;
    }

    writeln!(w, "</svg>")?;
    w.flush()?;
    Ok(())
}
