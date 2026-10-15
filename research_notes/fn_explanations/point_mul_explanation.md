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






