use crate::Triangle;

pub struct FrameBuffer {
    pub pixels: Vec<u8>, // RGB, 3 bytes per pixel
    pub width: u16,
    pub height: u16,
}

impl FrameBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            pixels: vec![0; width as usize * height as usize * 3],
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Load from an image crate DynamicImage
    pub fn from_image(img: &image::DynamicImage) -> Self {
        let rgb = img.to_rgb8();
        Self {
            width: rgb.width() as u16,
            height: rgb.height() as u16,
            pixels: rgb.into_raw(),
        }
    }

    pub fn save_png<P: AsRef<std::path::Path>>(&self, path: P) -> image::ImageResult<()> {
        image::save_buffer(
            path,
            &self.pixels,
            self.width as u32,
            self.height as u32,
            image::ColorType::Rgb8,
        )
    }

    /// Draws a horizontal line at row `y` from `x1` to `x2` with alpha blending.
    ///
    /// The line is drawn by blending the given RGB colour with the existing
    /// framebuffer contents using the formula:
    /// ```text
    /// new_pixel = old_pixel * (1 - alpha) + colour * alpha
    /// ```
    ///
    /// Lines outside the framebuffer bounds are clipped or ignored entirely.
    fn draw_hline(&mut self, x1: f32, x2: f32, y: f32, r: u8, g: u8, b: u8, alpha: f32) {
        let y = y as i32;
        if y < 0 || y >= self.height as i32 {
            return;
        }

        let mut x_start = x1.min(x2) as i32;
        let mut x_end = x1.max(x2) as i32;

        x_start = x_start.max(0);
        x_end = x_end.min(self.width as i32 - 1);

        let y = y as usize;
        let one_minus_alpha = 1.0 - alpha;

        for x in x_start..=x_end {
            let idx = (y * self.width as usize + x as usize) * 3;
            self.pixels[idx] = (self.pixels[idx] as f32 * one_minus_alpha + r as f32 * alpha) as u8;
            self.pixels[idx + 1] =
                (self.pixels[idx + 1] as f32 * one_minus_alpha + g as f32 * alpha) as u8;
            self.pixels[idx + 2] =
                (self.pixels[idx + 2] as f32 * one_minus_alpha + b as f32 * alpha) as u8;
        }
    }

    pub fn draw_triangle(&mut self, t: &Triangle) {
        let [(x1, y1), (x2, y2), (x3, y3)] = t.vertices;

        let (ax, ay) = (x1 as f32, y1 as f32);
        let (bx, by) = (x2 as f32, y2 as f32);
        let (cx, cy) = (x3 as f32, y3 as f32);

        let alpha = t.colour.alpha as f32 / 100.0;

        let dx1 = if by - ay > 0.0 {
            (bx - ax) / (by - ay)
        } else {
            bx - ax
        };
        let dx2 = if cy - ay > 0.0 {
            (cx - ax) / (cy - ay)
        } else {
            0.0
        };
        let dx3 = if cy - by > 0.0 {
            (cx - bx) / (cy - by)
        } else {
            0.0
        };

        let (mut sx, mut sy) = (ax, ay);
        let mut ex = ax;

        if dx1 > dx2 {
            while sy <= by {
                self.draw_hline(sx, ex, sy, t.colour.r, t.colour.g, t.colour.b, alpha);
                sy += 1.0;
                sx += dx2;
                ex += dx1;
            }
            ex = bx;
            while sy <= cy {
                self.draw_hline(sx, ex, sy, t.colour.r, t.colour.g, t.colour.b, alpha);
                sy += 1.0;
                sx += dx2;
                ex += dx3;
            }
        } else {
            while sy <= by {
                self.draw_hline(sx, ex, sy, t.colour.r, t.colour.g, t.colour.b, alpha);
                sy += 1.0;
                sx += dx1;
                ex += dx2;
            }
            sx = bx;
            sy = by + 1.0;
            while sy <= cy {
                self.draw_hline(sx, ex, sy, t.colour.r, t.colour.g, t.colour.b, alpha);
                sy += 1.0;
                sx += dx3;
                ex += dx2;
            }
        }
    }

    pub fn draw_triangles(&mut self, triangles: &[Triangle]) {
        self.clear();
        for t in triangles {
            self.draw_triangle(t);
        }
    }

    pub fn diff(&self, other: &FrameBuffer) -> i64 {
        debug_assert_eq!(self.pixels.len(), other.pixels.len());

        let mut d: i64 = 0;
        for (chunk_a, chunk_b) in self.pixels.chunks(3).zip(other.pixels.chunks(3)) {
            let dr = chunk_a[0] as i64 - chunk_b[0] as i64;
            let dg = chunk_a[1] as i64 - chunk_b[1] as i64;
            let db = chunk_a[2] as i64 - chunk_b[2] as i64;
            d += ((dr * dr + dg * dg + db * db) as f64).sqrt() as i64;
        }
        d
    }
}
