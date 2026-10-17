Basically, the function is implementing a classic cryptographic algorithm called Double-And-Add Algorithm (To be more specific, the left-to-right binary method, because it's `BigEndian`). It's the elliptic curve equivalent of "exponentiation by squaring".

So It's goal is to compute d = $a \times b$ (where `a` is our private key scalar, `b` the generator point `G`).

### Explanation about `mask`
The private key `a` is just an array of 21 bytes. To multiply the point, the algorithm needs to look at the private key bit by bit. From the `MSB` (Most Significant Bit, remember `BigEndian`) to the Least Significan Bit (`LSB`).

```c
for (mask = 0x80; mask != 0; mask >>= 1)
```

- `0x80` in hexadecimal is `10000000` in binary. It's a "mask" that isolates the very first bit of a byte.
- `mask >>= 1` shifts that `1` to the right by one position each iteration: `0x80` $\rightarrow$ `0x40` $\rightarrow$ `0x20` $\rightarrow$ `0x10` $\rightarrow$ `0x08` $\rightarrow$ `0x04` $\rightarrow$ `0x02` $\rightarrow$ `0x01`.
- `a[i] & mask` performs a bitwise `AND`. If the result Isn't `0`, that means the current bit is 1. If it's `0`, then `0` hehe.

**Example**
Let's make and example so then I can clarify myself more:

If `a[0]` is `0xA5` (in binary is 10100101):
1. `mask = 0x80` (`10000000`): `10100101 & 10000000 = 10000000` (Not 0, bit is 1)
2. `mask = 0x40` (`01000000`): `10100101 & 01000000 = 00000000` (Is 0, therefore `Bit = 0`)
3. `mask = 0x20` (`00100000`): `10100101 & 00100000 = 00100000` (Not 0, therefore `Bit = 1`)
... 
And so on.

### Double and add logic
After analyzing the mask loop, now let's look at what happens inside that:
```c
point_double(d, d);
if ((a[i] & mask) != 0)
	point_add(d, d, b);
```

Here is the core of the algorithm itself. For every single bit of the private key (`21 bytes * 8 bits = 168 iterations`), it does this:

1. **Always Doubles:** It doubles the current point `d`. (Geometrically, drawing a tangent line to the curve at point `d` and finding where it intersects again).
2. **But only Add with a certain condition:** If the current bit of the private key is `1`, it adds the base point `b` (the generator `G`) to `d`. (Geometrically, drawing a line between `d` and `G` and finding the intersection).

### The montgomery block
1. `bn_to_mon(k, ...)`
2. `point_mul(...)`
3. `point_from_mon(&ec_temp)`

Elliptic curve math requires modulo arithmetic. In standard math, modulo requires division, which is highly slow for CPUs
Montgomery multiplication is a math trick that replaces modular division with fast **bit shifting**. But in order to make the trick work, the numbers **must be transformed into Montgomery space first.**

- `bn_to_mon` pushes the private key `k` into Montgomery Form.
- `point_mul` does the 168 double and add operations. Because `k` is in the Montgomery Form, and `ec_G` was pre converted into Montgomery space before setting the curve with `ecdsa_set_curve`, all the internal additions and doublings stay in the Montgomery Space. Which is highly fast.
- `point_from_mon` pulls the final `ec_temp` coordinates out of Montgomery Space back into standard numbers, so then we can actually read the X and Y coordinates and use memcpy in order to copy into the public key buffer.

### What Montgomery Form Actually optimizes
Montgomery form does not change **Addition** or **Substraction**.
- `elt_add` and `elt_sub` work exactly the same way, whether the numbers are in standard form or Montgomery one.
- The code inside `bn_sub` is standard modular subtraction.

Montgomery form specifically optimizes **Multiplication**
In standard math, In order to compute ($A \times B$) ($mod \space N$), the CPU has to do this:
1. Multiply $A \times B$ (which creates a huge number)
2. Divide that huge number by $N$ in order to find the remainder. (Division is highly slow **specifically for large numbers**).

In Montgomery math, we transform the numbers so we can compute this:
$$
\large{(a \times B + m \times N) /R \space (mod \space N)}
$$
We don't necessary understand why this approach is better for the translation, since all the math is gonna get wrapped by the crate `BigUint` from Rust. But for the ones who are reading this documentation, here's a short summary of why this approach is better:

> [!NOTE] Why Optimize Multiplication in this way
> We constantly choose $R$ to be a power of 2 (e.g., $2^{160}$ or $2^{256}$), it depens on the curve size obviously)
> Dividing by a power of 2 is simply a right bit shift, In other words "Ignore the top words and keep the bottom words", so no slow division by N is needed. This algorithm, I looked it up, is called Montgomery Reduction (`REDC`).

## Design Decisions
### Design Decision: Differences between Field math and Curve Math
While `num-bigint`'s `BigUint` automatically handles low-level optimizations (e.g Montgomery Reduction) for **Field Arithmetic** (Integer × Integer mod PP), it has no concept of **Curve Arithmetic** (Integer × Point). 

Because of that, we can't just simply do a multiply function and that's it. We have to implement the `Double-And-Add` algorithm in order to dictate how points are combined, while delegating the modular arithmetic (such as `mod_add`, `mod_mul`, `mod_inv`) to `BigUint`.


> [!IMPORTANT] Just to clarify
> - **Our responsibility now:** Implement the control flow (bit-by-bit loop) that defined Elliptic Curve scalar multiplication
> - `BigUint`'s responsibility: Execute modular addition, subtraction, multiplication, and inversion efficiently without manually manage byte arrays, or Montgomery forms such as Legacy C code does.

### Design Decision: Double and add Algorithm
The bit by bit iteration, at first I though it was just another weird optimization tricks that I stumbled across during my reverse engineering journey of this tool. However this is not a simple "optimization", It's literally the fundamental definition of scalar multiplication on an elliptic curve. So because computers cannot do at a native level multiplying a scalar by a geometric point, we have to build the result in an iterative way.

The algorithm goes from the `MSB` (**Most Significant Bit**, that's why it starts with mask `0x80`), to the Least one (`LSB`). For every single bit, these rules apply:
1. **ALWAYS perform `point_double`:** This basically shifts the current accumulated point up by a power of 2.
2. **The conditional:** Perform `point_add` with the base point $G$ ONLY IF the current masked bit is `1` (remember this).
**Example:** Use the example above that explains these procedure by using 5 (`101`)

### Design Decision: Data Representation
Inside the original C implementation, the private key k was declared as a 21-byte array (`u8 k[0x15]`), with the first byte explicitly set to `0x00`.

- **C Context:** this was a defensive measure in order to prevent buffer overflows while performing modular arithmetic carries.
- **Rust Context:** `BigUint` is dynamically sized. Therefore a leading zero byte doesn't change the math value of the integer. As a result we can safely use a strict 20-byte array like `[u8;20]`. And `BigUint::from_bytes_be()` will handle the parsing safely.

### Final decision of Implementation
Based on everything I've explained above, the Rust implementation of `point_mul` will be like this:
1. Initialize the result point `d` to the Point at `Infinity`.
2. Convert `BigUint` scalar a into `BigEndian` byte slice, so we ensure we process from `MSB` to `LSB`
3. Iterate through EACH BYTE, and within that, iterate through the 8 bits using a descending mask (`0x80` down to `0x01`, remember).
4. Then without any sort of condition call `point_double_out_place` on each iteration.
5. But using a Condition we call `point_double_out_place` if `(byte & mask) != 0`.
6. Then rely entirely on the `mod_operations.rs` that I've written for the underlying math, trusting `BigUInt` to handle the rest to us in a safer manner.

#### Parameters from `point_mul`
What I had to do is delete a parameter inside the original C function, because I have everything inside the `psp_ctx` struct. Therefore I though it was unnecessary to send independently references that point into the same struct.  **So instead of using these parameters:**
```rust
pub fn point_mul(d: &mut Point, a: &BigUint, b: &mut Point, psp_ctx: &PSP_CTX)
```

**My decision is this one:**
```rust
pub fn point_mul(d: &mut Point, a: &BigUint, psp_ctx: &PSP_CTX)
```

**But why?:** Because inside `psp_ctx` we already have `ec_G`. Therefore we can just use it by calling `psp_ctx.g` and that's it.

> [!Caution] The thing above... I changed my mind
> Since I don't know whether this function si gonna get called inside another function and also for understand more the next code, I stick with this: `pub fn point_mul(a: &BigUint, b: &Point, psp_ctx: &PSP_CTX) -> Point`

**There's also another issue going on inside my code:**
```rust
pub fn ec_priv_to_pub(k: &mut BigUint, psp_ctx: &PSP_CTX) -> Point {
    let mut ec_temp: Point;
    point_mul(&mut ec_temp, k, &psp_ctx);
	// ... more code
```

Here we are sending an unitialized variable which causes the Rust compiler to argue with me. And since I was initializing `ec_temp` inside `point_mul` just to make the function portable and also just in case there's another function that calls it (which we don't know yet) now I got stuck at this. But there's a solution that I can do based on how it behaves `ec_temp` inside:

- It serves as a way to store the results that the main for-loop does while performing the mask `AND`'s and the iteration from the `a` pointer.

With that Idea in mind, I think I can just make `ec_temp` initialize itself inside `point_mul` rather than sending it from `ec_priv_to_pub`. With that I can just declare a variable called `ec_temp` and make a refactoring like **Replace function with Query**.

Since while lookin below in C, they perform a `memcpy` after making the result to point from Montgomery. So technically since I'm not gonna use Montgomery form, I think I can just make the result of `ec_priv_to_pub`, the result of `point_mul` itself. Result:

```rust
pub fn ec_priv_to_pub(k: &BigUint, psp_ctx: &PSP_CTX) -> Point {
    point_mul(k, &psp_ctx.g, &psp_ctx)
}
```

Wow, that was much simpler than I expected tbh. And respective change into `point_mul` might be this one:

```rust
pub fn point_mul(a: &BigUInt, b: &Point,  psp_ctx: &PSP_CTX) -> Point {
	let mut d = Point {
		x: BigUint::zero(),
		y: BigUInt::zero(),
		is_infinity: true,
	}
	// ... more code that I need to reverse engineer...
}
```







