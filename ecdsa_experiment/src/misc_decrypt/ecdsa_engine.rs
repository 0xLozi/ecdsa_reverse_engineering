use num_bigint::BigUint;
use num_traits::{FromBytes, Zero};

use crate::misc_decrypt::mod_operations::{mod_add, mod_inv, mod_mul, mod_sub};
use super::mod_operations::mod_square;
use crate::Error;

pub enum PrintableData<'a> {
    Bytes(&'a [u8]),
    BigUint(&'a BigUint),
}

#[derive(Clone)]
pub struct Point {
    pub x: BigUint,
    pub y: BigUint,
}

impl Point {
    pub fn infinity() -> Self {
        Point {x: BigUint::zero(), y: BigUint::zero()}
    }

    pub fn is_infinity(&self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }

    pub fn print_point(&self) {
        print!("X: ");
        for bytes in self.x.to_bytes_be() {
            print!("{:02X}, ", bytes);
        }
        print!("\n");
        print!("Y: ");
        for bytes in self.y.to_bytes_be() {
            print!("{:02X}, ", bytes);
        }
    }
}

pub struct Curve {
    pub p: BigUint,
    pub a: BigUint,
    pub b: BigUint,
    pub n: BigUint,
    pub g: Point,
}


impl Curve {
    // This builds a curve context. It replaces the C 'ecdsa_set_curve()' ('ec.c:378')
    // 
    // Differences form the C version:
    // - The parameters are stored in a 'Curve' value rather than the module-level globals 'ec_p' / 'ec_a' / 'ec_b' / 'ec_N' / 'ec_G'
    // - 'p', 'a', 'b' and the base point are expected in normal domain. 
    // C converts 'a', 'b' and 'G' to Montgomery form internally; this port does all the arithmetic
    // in the normal domain. So there's no convertion at all
    // - The inputs are validated (base point on the curve, non-zero order) and the function returns
    // 'Result' instead of always returning '0'
    // 
    pub fn new(p: &[u8], a: &[u8], b: &[u8], n: &[u8], gx: &[u8], gy: &[u8]) -> Result<Self, Error> {
        let p = BigUint::from_be_bytes(p);
        let n = BigUint::from_be_bytes(n);
        let a = BigUint::from_be_bytes(a);
        let b = BigUint::from_be_bytes(b);
        let g = Point {
            x: BigUint::from_be_bytes(gx),
            y: BigUint::from_be_bytes(gy),
        };

        // This is the prime field: It defines the size of the graph and it's limits where a point
        // can exist physically. So X or Y coordinate can't be larger than p.
        // Becase 
        if p.is_zero() || n.is_zero() {
            return Err(Error::InvalidParameter);
        }

        // Validate the generator point (ec_G). In the double-and-add loop, this is the 
        // point we add when a bit is 1. If it's Infinity (zero), the cryptography breaks 
        // because all public keys will evaluate to Infinity regardless of the private key.
        if g.is_infinity() {
            return Err(Error::InfinityBasePoint);
        }

        Ok(Curve { p, a, b, n, g })
    }
}


// ecdsa_engine::ec_priv_to_pub(k, &mut keypair.public_key, &psp_ctx);
pub fn ec_priv_to_pub(k: &BigUint, psp_ctx: &Curve) -> Point {
    point_mul(k, &psp_ctx.g, &psp_ctx)
}

pub fn point_mul(a: &BigUint, b: &Point, psp_ctx: &Curve) -> Point {

    let mut d = Point::infinity();

    let masks: [u8;8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

    for byte in a.to_bytes_be() {
        for mask in masks {
            point_double_in_place(&mut d, psp_ctx);
            if (byte & mask) != 0 {
                // point_add_in_place(&mut d, b, psp_ctx);
                let d_cloned = d.clone();
                point_add(&mut d, &d_cloned, b, psp_ctx);
            }
        }
    }
    d
}

// r = destination
// p = number A
// q = number B
// ctx = well, the context lmao
pub fn point_add(r: &mut Point, p_to_add: &Point, q: &Point, psp_ctx: &Curve) { 

    if p_to_add.is_infinity() {
        *r = q.clone();
        return;
    }

    if q.is_infinity() {
        *r = p_to_add.clone();
        return;
    }

    let mut u = mod_sub(&q.x, &p_to_add.x, &psp_ctx.p);

    if u.is_zero() {
        u = mod_sub(&q.y, &p_to_add.y, &psp_ctx.p);
        if u.is_zero() {
            point_double_out_place(r, &p_to_add, psp_ctx);
        } else {
            *r = Point::infinity();
        }
        return;
    }

    let rx = &mut r.x;
    let ry = &mut r.y;

    let mut t = mod_inv(&u, &psp_ctx.p); // t = 1/(qx-px)
    u = mod_sub(&q.y, &p_to_add.y, &psp_ctx.p); // u = qy-py
    let s= mod_mul(&t, &u, &psp_ctx.p); // s = (qy-py)/(qx-px)

    *rx = mod_square(&s, &psp_ctx.p); // rx = s*s
    t = mod_add(&p_to_add.x, &q.x, &psp_ctx.p); // t = px+qx
    *rx = mod_sub(rx, &t, &psp_ctx.p); // rx = s*s - (px+qx)

    t = mod_sub(&p_to_add.x, rx, &psp_ctx.p); // t = -(rx-px)
    *ry = mod_mul(&s, &t, &psp_ctx.p); // ry = -s*(rx-px)
    *ry = mod_sub(ry, &p_to_add.y, &psp_ctx.p); // ry = -s*(rx-px) - py
}

// Design Decision: point_double_out_place
// - Problem: Needed to avoid mutating input while updating result `r` without any sort of heap allocations.
// - First consideration: Cloning `BigUInt` (Safe, but caused unnecessary heap allocations and this functions will be called hundreds of times...).
// - Final Choice: Used immutable read references to `r` BEFORE mutating, achieving zero allocations!!!!
pub fn point_double_in_place(r: &mut Point, ctx: &Curve) {

    let pp = r.clone();
    let px = &pp.x;
    let py = &pp.y;

    if r.is_infinity() || r.y == BigUint::zero() {
        *r = Point::infinity();
        return;
    }

    // Calculation of the slope
    let mut t = mod_square(&r.x, &ctx.p); // t = px * px
    let mut s = mod_add(&t, &t, &ctx.p); // s = 2*px*px
    s = mod_add(&s, &t, &ctx.p); // s = 3*px*px
    s = mod_add(&s, &ctx.a, &ctx.p); // s = 3*px*px + a
    t = mod_add(&r.y, &r.y, &ctx.p); // t = 2*py
    t = mod_inv(&t, &ctx.p); // t = 1/(2*p.y)
    s =  mod_mul(&s, &t, &ctx.p); // s = (3*px*px+a)/(2*py)

    let rx = &mut r.x;
    let ry = &mut r.y;

    // elt_square(rx, s);  // rx = s*s
    *rx = mod_square(&s, &ctx.p); // rx = s*s

    // elt_add(t, px, px); // t = 2*px Here px  is pp.y,and pp = a struct point that contains the base value that was being sent. So I have to clone r. THAT'S WHY I COULND'T MANAGE TO IMPLEMENT THIS FUCKING Function...
    t = mod_add(px, px, &ctx.p); // t = 2*px

    // elt_sub(rx, rx, t); // rx = s*s - 2*px
    *rx = mod_sub(rx, &t, &ctx.p); // rx = s*s - 2*px

    // elt_sub(t, px, rx); // t = -(rx-px)
    t = mod_sub(px, rx, &ctx.p); // t = -(rx-px)

    // elt_mul(ry, s, t);  // ry = -s*(rx-px)
    *ry = mod_mul(&s, &t, &ctx.p); // ry = -s*(rx-px)

    // elt_sub(ry, ry, py);  // ry = -s*(rx-px) - py
    *ry = mod_sub(ry, py, &ctx.p);


}

// this is when r and p are different pointers. I do this because in Rust I cannot perform in-place doubling
pub fn point_double_out_place(r: &mut Point, p: &Point, ctx: &Curve) {
    let pp = p.clone();
    let px = &pp.x;
    let py = &pp.y;

    if p.is_infinity() || p.y == BigUint::ZERO {
        *r = Point::infinity();
        return;
    }

    // Here we calculate the slope
    let mut t = mod_square(&p.x, &ctx.p); // t = px * px
    let mut s = mod_add(&t, &t, &ctx.p); // s = 2*px*px
    s = mod_add(&s, &t, &ctx.p); // s = 3*px*px
    s = mod_add(&s, &ctx.a, &ctx.p); // s = 3*px*px + a
    t = mod_add(&p.y, &p.y, &ctx.p); // t = 2*py
    t = mod_inv(&t, &ctx.p); // t = 1/(2*p.y)
    s =  mod_mul(&s, &t, &ctx.p); // s = (3*px*px+a)/(2*py)

    let rx = &mut r.x;
    let ry = &mut r.y;

    *rx = mod_square(&s, &ctx.p); // rx = s*s

    //   elt_add(t, px, px); // t = 2*px
    t = mod_add(px, px, &ctx.p); // t = 2*px

    //   elt_sub(rx, rx, t); // rx = s*s - 2*px
    *rx = mod_sub(rx, &t, &ctx.p);

    //   elt_sub(t, px, rx); // t = -(rx-px)
    t = mod_sub(px, rx, &ctx.p);

    //   elt_mul(ry, s, t);  // ry = -s*(rx-px)
    *ry = mod_mul(&s, &t, &ctx.p);

    //   elt_sub(ry, ry, py);  // ry = -s*(rx-px) - py
    *ry = mod_sub(ry, py, &ctx.p);
}


pub fn print_slice(slice: PrintableData) {
    match slice {
        PrintableData::BigUint(b) => {
            let bytes = b.to_bytes_be();
            for byte in &bytes {
                print!("{:02X}", byte);
            } 
        },
        PrintableData::Bytes(slice) => {
            for byte in slice {
                print!("{:02X}", byte);
            }
        }
    }
    // Then we print a new line after the end
    println!();
}

    
fn elt_copy(dest: &mut BigUint, a: &BigUint) {
    // memcpy(d, a, 20); It copies 20, 20*8 = 160 => 160 refers to BigUint types. Therefore my hypothesis is that this copies BigUInt Types
    // and with this implementation I avoid buffer overflows 
    // Chose this over `*dest = a.clone()` since this way drops the old memory that dest was holding, then allocates new memory for te cloned value and assigns it to dest. The other way we can reuse the same memory instead.
    dest.clone_from(a);
}
