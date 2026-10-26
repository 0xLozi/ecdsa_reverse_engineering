When I completed the function I stumble across an nice optimization thing that I can perform inside that could make Rust faster than the old implementation (The old implementation I'm talking about is the Rust's old implementation). Let's look at this `cloning`:

```rust
let pp = p_to_add.clone();
let px = &pp.x;
let py = &pp.y;
```

**If you look at the entire code, the only moments that `px`, `py` or `pp` are written are these:**
```rust
None LMAO
```

That means that the references (`px`, `py`) pointing to `pp` **are never written**. And ALSO `p_to_add` is never written inside the code, because I'm sending `p_to_add` as an **IMMUTABLE REFERENCE**:

```rust
pub fn point_add_out_place(r: &mut Point, p_to_add: &Point, q: &Point, psp_ctx: &PSP_CTX) {
```

Therefore if it isn't written, why would I waste performance over duplicating an entire `Point` struct? This is highly unnefficient.

**That's how the implementation of `point_add()` inside `C` was made:**
```c
  pp = *p;
  qq = *q;

  px = pp.x;
  py = pp.y;
  qx = qq.x;
  qy = qq.y;
```

`pp` copies the entire `Point` struct rather than just using the reference itself. Therefore my **decision design** related to this manner is: **replace calls from `py` and `px` for `p.x & p.y`**.

**These are the calls from  `px`, `py` and `pp`:**
```
Line 193 -> if (point_is_zero(&pp)) {
Line 200 -> elt_copy(rx, px);
Line 205 -> elt_sub(u, qx, px);
Line 210 -> point_double(r, &pp);
Line 218 -> elt_sub(u, qy, py); // u = qy-py
Line 222 -> elt_add(t, px, qx); // t = px+qx
Line 225 -> elt_sub(t, px, rx); // t = -(rx-px)
Line 227 -> elt_sub(ry, ry, py);  // ry = -s*(rx-px) - py
```

As you can see, I'm always READING rather than writing. Therefore instead of copying the entire Point struct which is slow and unneficiente, I could just call `p` and the same goes to `q`

Original code:
```rust
pub fn point_add_out_place(r: &mut Point, p_to_add: &Point, q: &Point, psp_ctx: &PSP_CTX) {
    let mut u: BigUint;

    if p_to_add.is_infinity() {
        *r = q.clone();
        return;
    }

    if q.is_infinity() {
        *r = p_to_add.clone();
        return;
    }

    let qq = q.clone();

    let pp = p_to_add.clone();
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

    let mut t = mod_inv(&u, &psp_ctx.p); // t = 1/(qx-px)
    u = mod_sub(qy, py, &psp_ctx.p); // u = qy-py
    let s= mod_mul(&t, &u, &psp_ctx.p); // s = (qy-py)/(qx-px)

    *rx = mod_square(&s, &psp_ctx.p); // rx = s*s
    t = mod_add(px, qx, &psp_ctx.p); // t = px+qx
    *rx = mod_sub(rx, &t, &psp_ctx.p); // rx = s*s - (px+qx)

    t = mod_sub(px, rx, &psp_ctx.p); // t = -(rx-px)
    *ry = mod_mul(&s, &t, &psp_ctx.p); // ry = -s*(rx-px)
    *ry = mod_sub(ry, py, &psp_ctx.p); // ry = -s*(rx-px) - py
}
```

**New code:**
```rust
pub fn point_add_out_place(r: &mut Point, p_to_add: &Point, q: &Point, psp_ctx: &PSP_CTX) {
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
```

### Important things to say
Im not saying that the C implementation is wrong, this decision design related to create 2 functions that deals with "out_place" and "in_place" implementations. Each of them were comprised in only 1 function inside C. And these struct clones were necessary;.

```c
point_add(d, d, b);   // ec.c:241 — r and p are the same object
```

`r` aliases `p` in `point_mul`. Without the snapshot, the first `elt_square(rx, s)` overwrites `p->x`, and every later read of `px` (`elt_add(t, px, qx)`, `elt_sub(t, px, rx)`) would read corrupted data. `pp` exists precisely to survive the writes through `rx/ry`.

The copy is required because px/py are read **after** the result is written:
`elt_square(rx, s)` (221) overwrites p->x before `elt_add(t, px, qx)` (222) and
`elt_sub(t, px, rx)` (225) read it. py is worse: after `elt_mul(ry, s, t)` (226),
`elt_sub(ry, ry, py)` (227) subtracts the result from itself, so y collapses to 0.

#### Why one function is enough in Rust
The same object cannot be passed as both `&mut r` and `&p` because the compiler rejects `point_add(&mut d, &d, b, ctx)`. So the aliasing thing can only happen if the caller creates it, and the caller is the one that knows:
```rust
    let d_cloned = d.clone();
    point_add(&mut d, &d_cloned, b, psp_ctx);
```

That is C `pp = *p`, moved to the call site. The function is identical in both cases and does no cloning itself; a call whose destination is also an operand pays one `Point` clone (two heap allocations), and every other call pays nothing. `point_add_in_place` is therefore dropped: one implementation, and the caller decides whether a snapshot is needed.

Therefore this file is gonna get called "point_add" rather than "point_add_out_place" And I'm gonna delete "point_addition_in_place" because I don't think it's neccesarry now.






