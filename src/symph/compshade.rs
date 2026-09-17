// compshade.rs ----------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

// Wang hash — fast, deterministic integer hash for pseudo-random number generation.
// Modeled directly from Trellis symph/compshade.h.
#[inline]
pub const fn  WangHash( mut seed: u32) -> u32
{
    seed = ( seed ^ 61) ^ ( seed >> 16);
    seed = seed.wrapping_mul( 9);
    seed = seed ^ ( seed >> 4);
    seed = seed.wrapping_mul( 0x27d4_eb2d);
    seed = seed ^ ( seed >> 15);
    seed
}

//-------------------------------------------------------------------------------------------------

// HashToFloat — maps a hash value to a float in [0.0, 1.0) by masking to 24 bits and dividing by 2^24.
#[inline]
pub const fn  HashToFloat( h: u32) -> f32
{
    ( h & 0x00FF_FFFF) as f32 / 16777216.0
}

//-------------------------------------------------------------------------------------------------

// Collatz — computes the number of Collatz steps for an integer. Returns u32::MAX on 0 or overflow.
#[inline]
pub const fn  Collatz( mut n: u32) -> u32
{
    if n == 0
    {
        return u32::MAX;
    }
    let  mut i = 0;
    while n != 1
    {
        if n.is_multiple_of( 2)
        {
            n /= 2;
        } else
        {
            if n >= 0x5555_5555
            {
                return u32::MAX;
            }
            n = 3 * n + 1;
        }
        i += 1;
    }
    i
}

//-------------------------------------------------------------------------------------------------

// Element-wise Compute Kernels
#[inline]
pub fn  DoubleElem( idx: usize, data: &mut [f32])
{
    if idx < data.len()
    {
        data[idx] *= 2.0;
    }
}
#[inline]
pub fn  VectorAddElem( idx: usize, a: &[f32], b: &[f32], out: &mut [f32])
{
    if idx < a.len() && idx < b.len() && idx < out.len()
    {
        out[idx] = a[idx] + b[idx];
    }
}
#[inline]
pub fn  CollatzElem( idx: usize, inp: &[u32], out: &mut [u32])
{
    if idx < inp.len() && idx < out.len()
    {
        out[idx] = Collatz( inp[idx]);
    }
}
#[inline]
pub fn  PointCloudElem( idx: usize, out: &mut [f32])
{
    let  base = idx * 4;
    if base + 3 < out.len()
    {
        let  hx = WangHash( ( idx * 3) as u32);
        let  hy = WangHash( ( idx * 3 + 1) as u32);
        let  hz = WangHash( ( idx * 3 + 2) as u32);
        let  x = HashToFloat( hx) * 40.0 - 20.0;
        let  y = HashToFloat( hy) * 40.0 - 20.0;
        let  z = HashToFloat( hz) * 40.0 - 20.0;
        out[base] = x;
        out[base + 1] = y;
        out[base + 2] = z;
        out[base + 3] = 1.0;
    }
}
