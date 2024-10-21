
mod perlin_noise {
    use rand::Rng;
    use cgmath::{Vector2, dot, InnerSpace};

    //https://en.wikipedia.org/wiki/Perlin_noise

    fn interpolate(a0: f32, a1: f32, w: f32) -> f32 {
        (1.0 - w) * a0 + w * a1
    }

    fn dot_grid_gradient(ix: i32, iy: i32, x: f32, y: f32) -> f32 {
        let dx = x - ix as f32;
        let dy = y - iy as f32;
        let random = PERMUTATION[(ix + PERMUTATION[iy as usize as usize] as i32) as usize as usize];
        let random = random as f32 / 255.0;
        let gradient = Vector2::new(dx, dy);
        let gradient = gradient.normalize();
        dot(gradient, Vector2::new(x, y)) * random
    }

    fn perlin(x: f32, y: f32) -> f32{
        let x0 = x.floor() as i32;
        let x1 = x0 + 1;
        let y0 = y.floor() as i32;
        let y1 = y0 + 1;

        let sx = x - x0 as f32;
        let sy = y - y0 as f32;

        let n0 = dot_grid_gradient(x0, y0, x, y);
        let n1 = dot_grid_gradient(x1, y0, x, y);
        let ix0 = interpolate(n0, n1, sx);

        let n0 = dot_grid_gradient(x0, y1, x, y);
        let n1 = dot_grid_gradient(x1, y1, x, y);
        let ix1 = interpolate(n0, n1, sx);

        interpolate(ix0, ix1, sy) // final noise value, will return a value between -1 and 1, in order to convert it to 0-1, multiply by 0.5 and add 0.5
    }

    const PERMUTATION: [u8; 256] = [
        151, 160, 137,  91,  90,  15, 131,  13, 201,  95,  96,  53, 194, 233,   7, 225,
        140,  36, 103,  30,  69, 142,   8,  99,  37, 240,  21,  10,  23, 190,   6, 148,
        247, 120, 234,  75,   0,  26, 197,  62,  94, 252, 219, 203, 117,  35,  11,  32,
        57, 177,  33,  88, 237, 149,  56,  87, 174,  20, 125, 136, 171, 168,  68, 175,
        74, 165,  71, 134, 139,  48,  27, 166,  77, 146, 158, 231,  83, 111, 229, 122,
        60, 211, 133, 230, 220, 105,  92,  41,  55,  46, 245,  40, 244, 102, 143,  54,
        65,  25,  63, 161,   1, 216,  80,  73, 209,  76, 132, 187, 208,  89,  18, 169,
        200, 196, 135, 130, 116, 188, 159,  86, 164, 100, 109, 198, 173, 186,   3,  64,
        52, 217, 226, 250, 124, 123,   5, 202,  38, 147, 118, 126, 255,  82,  85, 212,
        207, 206,  59, 227,  47,  16,  58,  17, 182, 189,  28,  42, 223, 183, 170, 213,
        119, 248, 152,   2,  44, 154, 163,  70, 221, 153, 101, 155, 167,  43, 172,   9,
        129,  22,  39, 253,  19,  98, 108, 110,  79, 113, 224, 232, 178, 185, 112, 104,
        218, 246,  97, 228, 251,  34, 242, 193, 238, 210, 144,  12, 191, 179, 162, 241,
        81,  51, 145, 235, 249,  14, 239, 107,  49, 192, 214,  31, 181, 199, 106, 157,
        184,  84, 204, 176, 115, 121,  50,  45, 127,   4, 150, 254, 138, 236, 205,  93,
        222, 114,  67,  29,  24,  72, 243, 141, 128, 195,  78,  66, 215,  61, 156, 180
    ];

}