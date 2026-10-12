use std::{fmt::Error, sync::Arc};
use num_traits::{FromBytes, Zero};
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
    pub is_infinity: bool,
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
            is_infinity: false 
        }
    }
}

// ecdsa_engine::ec_priv_to_pub(k, &mut keypair.public_key, &psp_ctx);
pub fn ec_priv_to_pub(k: [u8;20], Q: &Point, psp_ctx: &PSP_CTX) -> Result<(), Error> {
    let mut ec_temp = Point {
        x: BigUint::zero(),
        y: BigUint::zero(),
        is_infinity: true
    };

    let k_scalar = BigUint::from_be_bytes(&k);
    point_mul(&mut ec_temp, &k_scalar, &psp_ctx.g, &psp_ctx);

    Ok(())
}

pub fn point_mul(d: &mut Point, a: &BigUint, b: &Point, psp_ctx: &PSP_CTX) {
    // point_zero(d); We already did this on "ec_priv_to_pub"
    // for i in (1..160).rev() {
    //     point_double(d, d, psp_ctx);
    //     if a.bit(i) {
    //         point_add(d, d, b);
    //     }
    // }
}

// this is when r and p are different pointers. I do this because in Rust I cannot perform in-place doubling
pub fn point_double_out_place(r: &mut Point, p: &Point, ctx: &PSP_CTX) {
  if p.is_infinity || p.y == BigUint::ZERO {
    *r = Point { x: BigUint::zero(), y: BigUint::zero(), is_infinity: true };
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

  // rx = r->x
  // Then we Calculate the new coordinates
  let rx = &mut r.x;
  *rx = mod_square(&s, &ctx.p);
  t = mod_add(&p.x, &p.x, &ctx.p);
  // I cant send &r.x as "a" because I'm borrowinng as a mutable reference at line 74. Therefore I'm gonna use rx instead
  *rx = mod_sub(&rx, &t, &ctx.p);

  t = mod_sub(&p.x, &r.x, &ctx.p);
  let ry = &mut r.y;
  *ry = mod_mul(&s, &t, &ctx.p);
  *ry = mod_sub(&ry, &p.y, &ctx.p);
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