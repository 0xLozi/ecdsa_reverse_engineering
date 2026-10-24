use num_traits::{FromBytes, ToBytes, Zero};
use crate::{BigUint, misc_decrypt::mod_operations::{mod_add, mod_inv, mod_mul, mod_sub}};
use super::mod_operations::{mod_square};

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
}

pub struct PSP_CTX {
    pub p: BigUint,
    pub a: BigUint,
    pub b: BigUint,
    pub n: BigUint,
    pub g: Point,
}

// Now we have the curves that we're gonna use just here.
pub fn ecdsa_set_curve(p: &[u8], a: &[u8], b: &[u8], n: &[u8], gx: &[u8], gy: &[u8]) -> PSP_CTX {
    PSP_CTX { 
        p: BigUint::from_be_bytes(p), 
        a: BigUint::from_be_bytes(a), 
        b: BigUint::from_be_bytes(b), 
        n: BigUint::from_be_bytes(n), 
        g: Point { 
            x: BigUint::from_be_bytes(gx), 
            y: BigUint::from_be_bytes(gy), 
        }
    }
}

// ecdsa_engine::ec_priv_to_pub(k, &mut keypair.public_key, &psp_ctx);
pub fn ec_priv_to_pub(k: &BigUint, psp_ctx: &PSP_CTX) -> Point {
    point_mul(k, &psp_ctx.g, &psp_ctx)
}

pub fn point_mul(a: &BigUint, b: &Point, psp_ctx: &PSP_CTX) -> Point {

    let mut d = Point::infinity();

    let masks: [u8;8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

    for byte in a.to_bytes_be() {
        for mask in masks {
            point_double_in_place(&mut d, psp_ctx);
            if (byte & mask) != 0 {
                point_add_in_place(&mut d, b, psp_ctx);
            }
        }
    }

    d
}

// r = destination
// p = number A
// q = number B
// ctx = well, the context lmao
pub fn point_add(r: &mut Point, p_to_add: &mut Point, q: &Point, psp_ctx: &PSP_CTX) {
    let s: BigUint;
    let t: BigUint;
    let mut u: BigUint;

    if p_to_add.is_infinity() {
        *r = q.clone();
        return;
    }

    if q.is_infinity() {
        *r = p_to_add.clone();
        return;
    }

    let pp = p_to_add.clone();
    let qq = q.clone();

    let px = &pp.x;
    let py = &pp.y;
    let qx = &qq.x;
    let qy = &qq.y;

    let rx = &mut r.x;
    let ry = &mut r.y;

    u = mod_sub(qx, px, &psp_ctx.p);

    if u.is_zero() {
        u = mod_sub(qy, py, &psp_ctx.p);
        if u.is_zero() {
            point_double_out_place(r, &pp, psp_ctx);
        } else {
            *r = Point::infinity();
        }
        return;
    }

}

pub fn point_add_in_place(r: &mut Point, b: &Point, psp_ctx: &PSP_CTX) {
    // WE HAVE TO TRANSLATE THIS USING THE OUT_PLACE IMPLEMENTATION AS A TEMPLATE


}

// Design Decision: point_double_out_place
// - Problem: Needed to avoid mutating input while updating result `r` without any sort of heap allocations.
// - First consideration: Cloning `BigUInt` (Safe, but caused unnecessary heap allocations and this functions will be called hundreds of times...).
// - Final Choice: Used immutable read references to `r` BEFORE mutating, achieving zero allocations!!!!
pub fn point_double_in_place(r: &mut Point, ctx: &PSP_CTX) {

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
pub fn point_double_out_place(r: &mut Point, p: &Point, ctx: &PSP_CTX) {
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
