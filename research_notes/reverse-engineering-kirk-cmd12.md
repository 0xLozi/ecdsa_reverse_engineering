First let's see what it does this function:

```cpp
ecdsa_set_curve(ec_p,ec_a,ec_b2,ec_N2,Gx2,Gy2);
```

It calls a subroutine called `ecdsa_set_curve`.
Then after that they do the next things:

```cpp
  k[0] = 0;
  kirk_CMD14(k+1,0x14);
  ec_priv_to_pub(k, (u8*)keypair->public_key.x);
  memcpy(keypair->private_key,k+1,0x14);
```

**Which it does this:**
1. Assigns `k[0]` = 0
2. Calls another routine called `kirk_CMD14` and it sends k with shifting of 1, maybe for skipping the first byte from the array?
3. And then it sends `0x14`, which I guess is the size of `k` with scrolling `1`. I think that because the size of `k` is `0x15`
4. Calls another routine called `ec_priv_to_pub(k, (*u8)keypair->public_key.x);` , possibly converting a private key to a public one? What I see is that they cast the pointer that points to the x variable from public_key as an `u8` pointer. But I don't know exactly why they do this, wouldn't be better to just send `public_key.x` into the function rather than cast that pointer into an `*u8`?.
   `x` is an `u8 x[0x14]` array, from a struct called `ECDSA_POINT`, and is being called by another struct (`KIRK_CMD12_BUFFER`). Maybe instead of sending a reference of `[u8;0x14]`, maybe they just want to send an `u8*` so then the references are compatible because the sizes of `k` and `public_key.x` are highly similar?

##### About the 4th point
While doing a research, I stumble across this explanation about why they did the casting in the first place:

The `u8*` cast is something that was classic back then in order to bypass compiler warnings about buffer overflows.

**Explanation**
In the `ECDSA_POINT` struct, `x` is defined as a 20-byte array (`u8 x[0x14]`)
However, an **Eliptic Curve Public Key** consists of two coordinates: `X` (20 bytes) and `Y` (20 bytes). And the `ec_priv_to_pub` function calculates both of them, meaning that it needs to write 40 bytes of data to the pointer I give it.

So if the programmer just passed `keypair->public_key.x` normally, a smart C compiler would look at the code and throw us a warning that we are passing an array of 20 bytes into a function that writes 40. As a result it will cause a buffer overflow.

But the C programmer knew this: Inside the struct, `y` sits exactly next to `x` in memory!!!

```cpp
typedef struct {
    u8 x[0x14]; // Bytes 0 to 19
    u8 y[0x14]; // Bytes 20 to 39
} ECDSA_POINT;
```

So by casting x to a raw `(u8*)` pointer, the programmer lets behind the `0x14` size information and he just tells the compiler to stop treating it like a `20-byte` array and just treat it as a raw starting address.

So the function writes the first `20` bytes, filling `x`, and then it keeps writing past the boundary, not making an "overflow" but rather "overflowing" into `y`. Did that make sense?. And that is highly performant, but incredibly dangerous if the struct layout ever changes.

#### Back into `ecdsa_set_curve`
What I see is that `ecdsa_set_curve` is not inside `kirk_engine.c`. Therefore I'm going to search if it's a library.

```
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "kirk_engine.h"
#include "AES.h"
#include "SHA1.h"
```


> [!NOTE] What I know about ecdsa
> **ECDSA** stands for Elliptic Curve Digital Signature Algorithm, a cryptographic scheme used to generate and verify digital signatures using elliptic curve cryptography (ECC)

**So my guess is that the function might be inside:**
- `AES.h`
- `SHA1.h`

**Results:**
- `AES.h` $\rightarrow$ Not inside
- `SHA1.h` $\rightarrow$ not inside

This is weird, since inside `kirk_engine.h` we see this:
```cpp
// Prototypes for the Elliptic Curve and Big Number functions
int ecdsa_get_params(u32 type, u8 *p, u8 *a, u8 *b, u8 *N, u8 *Gx, u8 *Gy);
int ecdsa_set_curve(u8* p,u8* a,u8* b,u8* N,u8* Gx,u8* Gy);
void ecdsa_set_pub(u8 *Q);
void ecdsa_set_priv(u8 *k);
```

**While doing my research, I've found lots of things that I wanna point out:**
- **The Architecture:** `kirk_engine.h` makes a promise that the math functions exist, but the actual heavy math is isolated in a separate C file that is being linked during compilation.
[[How it Works GCC]]

- **The Math:** You set the curve rules and starting point -> you generate a random number (Private Key) -> you mathematically bounce along the curve that many times -> where you land is the Public Key (That's what I see).

- **PSP** uses `160-bit` numbers, and here's why we need `BigUint` and how **CPU** limit works:
	- A 64-bit CPU possesses hardware registers and an Arithmetic Logic Unit (`ALU`) that are exactly 64 bits wide. When the CPU performs native arithmetic (like an `ADD` or `MUL` instruction), it operates on these registers in a single clock cycle. The maximum unsigned integer value a 64-bit register can hold without overflowing is $2^{64} - 1$ (approximately 18.4 quintillion).
	
	- Rust is a modern language, so it gives us the `u128` type. However this tells the compiler to automatically duct-tape two 64 bit hardware buckets together to hold a 128-bit number.
	
	- Sony's cryptography uses numbers that are 160 bits long (the 20-byte arrays that we saw: `[u8;0x14]`).
	  So 160 bits is too big for the 64-bit bucket. It is even too big for Rust's `u128` bucket. So If we try to force a `160-bit` number into a standard Rust number type, the top `32 bits` spills over the edge and are permanently lost, destroying the cryptographic key.
	  
	- **Solution:** `BigUint`:
	  The `num-bigint` crate's `BigUint` type solves this by storing the 160-bit number inside a dynamically sized array (e.g., a `Vec<u32>` or `Vec<u64>`) allocated on the heap. When you request a multiplication (`A * B`), the crate does not use a single CPU `MUL` instruction. Instead, it executes an algorithm that iterates through the underlying arrays, multiplying the individual 32-bit or 64-bit chunks in sequence, and propagating the carry flags across the array boundaries purely in software.

---
#### How can I be sure that the legacy implementation is forcing the CPU to simulate 160-bit math
If this code were running on a machine with native 160-bit hardware registers, it would look completely different. Here's the difference:

**Data Types (`u8*`):**
If we look at the function arguments: `u8* p`, `u8* a`. The programmer is passing pointers to arrays of bytes (`u8`). So if the CPU natively understood 160 bit numbers, then C would have a built-in primitive type for it (like for example `uint160_t`), just like it has `uint64_t`. The function signature would be simply int `ecdsa_set_curve(uint160_t p uint160_t a...)`. Because the CPU doesn't have a `160 bit` type, the programmer was forced to cut the number into twenty blocks of `8bit` ($8*20 = 160$).

**Using `memcpy` rather than `=`:**
Let's look at `memcpy(ec_p, p, 20);`
When we assign a value to a native CPU register, you can just use the equals sign right? (e.g, `x = y`). The CPU copies the memory in one hardware tick.
You only use `memcpy` when you are being forced to copy a data structure byte by byte because it's too large for the CPU to move in one piece.

**The `bn_` prefix and also Length Arguments**
If we look at `bn_to_mon(ec_a, ec_p, 20);`
- `bn_` is highly possible that it means **Big Number**. Because of the naming convention in **C** for precision arithmetic software.
- **The 20 argument:** The programmer has to explicitly pass the number **20** into the function to tell the for loop inside `bn_to_mon`.

#### Why I think BigUint is the best case scenario
The pspdecrypt C code is a software emulator. So it uses arrays of bytes (u8) and manual for loops (bn_to_mon) to mathematically fake a hardware chip of 160 bits.
My Rust Implementation will do exactly the same thing but safely. Becuase in Rust, `BigUints` is just an array of smaller CPU numbers that are bundled with optimized for-loops in order to perform the math. Therefore by using `BigUint`, I'm replacing manual c arrays with safe Rust arrays.

## Steps for verification of code implementation
This is what I'm going to do in order to compare results with the c implementation and my custom Rust Implementation:


#### First: Isolate C Code
I'm going to isolate a few files from the original repo in order to make it compile independently:
- `ec.c` (for the curve logic)
- `bn.c` (the **Big Number** math, since `ec.c` uses `bn_to_moon`)
- The header files that define the structs (`kirk_engine.h` or smt similar).
While doing this I also had to put `kirk_engine.c` because there was a function from `generate_ecdsa` that uses `kirk_cmd14` and that function is implemented inside `kirk_engine.c`. And also `AES`, `SHA1`, etc. Basically almost all the files inside the root folder.

#### Write the C Tests
I'm going to create a file called `test_main.c`
Inside main(), I'm not gonna use random number. I'm going to use a hardcoded byte arrays for everything
1. I define Sony curve arrays
2. Define a fake hardcoded Private Key with 20 bytes
3. I call ecdsa_set_curve and ec_priv_to_pub
4. Then I write a simple printf loop in order to output the final Public Key X and Y coordinates as a clean Hexadecimal string.
After that, I compile it and run it. Then I save the Hex string 

I have to put `k` as 21 bytes since the `double-addition` and that stuff. The process itself could lead to math results that can go past the boundaries limits, even though the desired key is `0x14` bytes

**And the result while executing the program after compiling it is:**
```plaintext
Now we are setting the curve
PRIVATE KEY:BFE176D6918D66FA4D8E525C97CB3A2D3FC68BA9
Public Key (X)
851FCAC5FD7E64A98B09748C8A82271DB80FAF56
Public Key (Y)
ADE57264EC85B045CD1BC365D896B09D186C7B99
```


---
## Write the Rust code
Now we have to replicate this code and reach the same results in rust. These are the steps that I'm going to do:

### 1. Create a cargo project
1. Create a `Cargo Project`
2. Import `num-bigint` crate
3. Implement in Rust!!!

**Theres another crate that Im missing because of this:**
Because Elliptic Curve math cannot do standard multiplication, it relies on a loop called "Double-and-Add" to calculate the final Public Key.

Therefore, I need zero for the accumulator, just like when I write a basic for loop that I accumulate the sum of the total of an array of numbers like this:
```c
int sum = 0; // We must start with 0 here
for (int i = 0; i < 10; i++) {
	sum = sum + array[i];
}
```

And the Elliptic curve multiplication is essentially a giant `for loop` that adds points together that are based on the bits in the private key

So, to start a loop, I need a blank var that acts as our `sum = 0`

Therefore I also need `num-traits` crate that provides a standard math behavior (like Zero and One) to complex types.
So by pulling `num_traits::Zero`, I'm unlocking these abilities that `BigUint` can do:
- **Creation:** I can spawn a zero massive number using `BigUint::zero()` (which replaces C's `memset`). I used this in the placeholder to make the code compile.

- **Checking:** It gives you the `.is_zero()` method. We will absolutely need to write `if public_key.x.is_zero()` in our cryptography logic to prevent the division-by-zero crashes I saw earlier in C.

---
### Performing the translation
While performing the translation, I made sure to not use global variables since in Rust is a bad habit. Created a `PSP_CTX` struct that stores the respective curves. But first I have to explain something important: **The Montgomery form**.

Montgomery form is used to optimize multiplication.
The real enemy of a processor is modular multiplication: `(A * B) % P`. To find the remainder (`%P`), the CPU essentially has to perform long division. Division is the slowest and most agonizing instruction a CPU can execute.
And here's how exactly the C engine is preparing for.

##### What is the C engine is preparing for then?
To calculate our Public Key, the engine has to take the Base Point (`ec_G`) and mathematically multiply it by our Private Key (160 bits). It does this by looping 160 times, constantly performing **Point Additions** and **Point Doublings**.

**Why `% P` is the Ultimate CPU Bottleneck**
To understand the Montgomery trick, let's look closely at modular multiplication:
$$
A \cdot b \pmod P 
$$
Multiplying $A \cdot B$ is fast. But the problem is the$\pmod P$.
I order to find the remainder of a number, the CPU must perform long division. So if I ask to calculate $54321 \pmod{100}$, it is instant (just look at the last two digits... `21`). But if I ask you to calculate this: $54321 \pmod{97}$, you just have to pull out a piece of paper and perform **LONG DIVISION**.

Well, computers work somewhat the same in binary. If a computer divides by a power of 2 (e.g. `10000` in binary), it doesn't do math; it just shifts the bits to the right (`>>`). So a bit-shift takes exactly **1 CPU clock cycle**.

BUT our prime ceiling (which is $P$) is not a clean power of `2`, it's a cryptographic (AND ALSO UGLY) prime number. And because of that, the CPU cannot do the **bit shifting**. Therefore it's forced to do **digital long division** on massive numbers of 160 bits. Doing this hundreds of times per key generation would explode the **PSP**.

#### The Montgomery Magic Trick
In 1985, Peter Montgomery discovered a mathematical loophole. He realized that if you multiply all your starting numbers by a massive power of 2 (called $R$), you "teleport" those numbers into a parallel mathematical universe called the **Montgomery Domain**.

<u>Here's the magic of the Montgomery Domain</u>:
When you multiply two numbers in this domain, the math equation naturally shifts. So Instead of having to divide by the ugly $P$ in order to find the remainder, the math allows you to divide by $R$ instead.
And because $R$ was specifically chosen to be a clean power of 2, the CPU can now use 1 cycle bit shifts rather than performing a **long division**.

**And this is exactly what `ecdsa_set_curve` is doing in the C code:**
1. Takes the curve parameters (a, b) and the starting point (G).
2. Calls `bn_to_mon` tu multiply them by $R$, teleporting them into the **Montgomery Domain**.
3. Now, the Double and Add loop can run 160 times, using fast bit-shifts.
4. At the very end of the engine, the final Public Key coordinates are shifted back into the normal world so they can be read.

##### Why we are skipping this in Rust
Modern CPUs are incredibly faster than the PSP, and the Rust `num-bigint` crate has highly optimized division algorithms (e.g. Barett reduction) already built into it's `%` operator. So by skipping the **Montgomery Teleportation**, we keep our Rust code clean, safe and mathematically readable!!!.

#### Back to Rust
I created a `PSP_CTX` in order to wrap the global variables from C into a Rust struct so I don't have to deal with global variables in Rust. Also created another struct to store `GY & GX` called `Point`, and added an `is_infinite` so then I can skip doing math using `(0, 0)`, or else it will explode because it violates the curve's geometry.
**First let's call the function:** `ec_priv_to_pub`. And because I don't have to use **Montgomery**, I don't have to initialize `k` (the key) as 21, rather I can do it with 20 because I don't need that extra bit if the calculation goes out of the boundaries.

```rust
// ec_priv_to_pub(k, (u8*)keypair->public_key.x);
ecdsa_engine::ec_priv_to_pub(k, &keypair.public_key.x, &psp_ctx);
```

**I think it's good, but something seems off to me:**
I know why they cast the `public key x` into a `u8` pointer. I Think it was because they needed also "y". Therefore they can skip **compilation warnings** and then go past the boundaries because they kow that, after the boundary limits of x, next to it there's "y". **Therefore I think I might change it to this:**

```rust
ecdsa_engine::ec_priv_to_pub(k, &mut keypair.public_key, &psp_ctx);
```

**And with that I can access to both variables: `x & y`**

### Translating `ecdsa_priv_to_pub`
This is the original C code function:

```c
// Modified from original to support kirk engine use - July 2011
void ec_priv_to_pub(u8 *k, u8 *Q)
{
  struct point ec_temp;
  bn_to_mon(k, ec_N, 21);
  point_mul(&ec_temp, k, &ec_G);
  point_from_mon(&ec_temp); 
  //bn_from_mon(k, ec_N, 21);
  memcpy(Q,ec_temp.x,20);
  memcpy(Q+20,ec_temp.y,20);
}
```

<u>These are the steps that the function does:</u>
1. Creates a `struct point` called `ec_temp`
2. Calls `bn_to_mon` and sends: `k, ec_N, 21`
3. Calls `point_mul()` and sends: `&ec_temp`
4. Calls `point_from_mon()` and sends `&ec_temp`
5. Uses `memcpy` and sends these parameters: `(Q, ec_temp.x, 20);`
6. Uses `memcpy` and sends these parameters: `(Q+20, ec_temp.y, 20);`

### Explanation about each points
##### **1st point**
`struct point ec_temp;` is a physical dot on the elliptic curve graph. In **C**, a `struct point` is just a data structure that holds an `x` byte array and a `y` byte array.

##### **2nd point**
**`bn_to_mon(k, ec_N, 21);`**
Just like in the setup function, It's teleporting the Private Key (k) into the Montgomery domain that we've just talked before, so it can do fast math.
>Let's notice that it uses `ec_N` rather than `ec_P`, a quirk of ECC math, but since I'm using Rust `BigUInt`, I can ignore this step

**`ec_p` (The Prime Field)**
- **What it limits:** The X and Y coordinates.
- **What this means:** `P` defines the literal size of the graph. It dictates exactly where a point can physically exists. No `X` or `Y` coordinate can ever be larger than `P`.
- **And where it's used:** **Point Addition** and **Point Doubling**. When you draw the line to find a new point, all the geometry math (`lambda`, `x2`, `y2`) is calculated modulo `P`.

**`ec_N` (The Curve Order)**
- **What it limits:** The Private Key (and all scalars/multipliers)
- **What this means:** `N` is the total number of valid dots on the entire curve. So If you take the **Base Point** ($G$) and add it to itself exactly `N` times, you'll perfectly loop back around and hit the Point at **Infinity**. Because the curve resets at `N`, no private key everd need to be larger than `N`.

- **Where it is used:** Operations involving the Private Key. If you have a private key that is mathematically larger than N, it just wraps around. A private key of N + 1 generates the exact same public key as a private key of 1.

**In short terms: I can SKIP this (delete entirely bn_to_mon on Rust)**

##### **3rd point**
`point_mul(&ec_temp, k, &ec_G);`
**This is the Double-and-Add loop!**
The C programmer didn't write the 160-loop directly inside this function. They hid it inside another function called `point_mul` (Point Multiplication).
**Basically this does:** Take the Base Point (`ec_G`), multiply it by the **Private Key** using the **Double-And-Add** loop, and save the final answer into the `ec_temp` container.
##### **4th point**
`point_from_mon(&ec_temp);`
This is the **Montgomery Reversal**. Now that the loop is finished and the math is done, the answer inside `ec_temp` is still trapped in the **Montgomery parallel universe**. If you printed it now, it would be wrong. This function shifts it back into normal mathematics.
##### **5th point**
`memcpy(Q, ec_temp.x, 20);` and `memcpy(Q+20, ec_temp.y, 20);`
This is the pointer hack that I've predicted earlier, which was the `*u8` pointer cast!!! `Q` is the output buffer. It copies the 20 byte of the `X` coordinate into the front of the buffer, and then uses `Q+20` to skip ahead of the `X` coordinate into the front of the buffer, and then uses Q+20 to skip ahead and shove the 20 bytes of the Y coordinate directly behind it.

#### Now let's see which functions leaves and which ones remains

**Which ones leaves:**
1. `bn_to_mon(k, ec_N, 21);` I think it isn't necessary at this point, because I don't wanna convert "k" into the Montgomery form, because I have `BigUint160`
2. `point_from_mon(&ec_temp);` This is the Montgomery Reversal. This was used when the math was done. And since I'm not gonna use Montgomery, I'm gonna delete it.
3. `struct point ec_temp;` (as an uninitialized C variable) **Deleted.**

**What Remains:**
- **`point_mul(...)`** **Stays but rewritten**. This is where the Double-and-Add loop lives. This is the engine that actually generates the public key.

- **`memcpy` (The Output)****. Stays (but made memory-safe). I still need to extract the final $X$ and $Y$ bytes from the `BigUint` coordinates and write them to the final `PublicKeyStruct`.

---

#### More information about decision making `ec_priv_to_pub`
I've recently discover the purpose of `struct point *d`:
What I do first at `ec_priv_to_pub` is to initialize a new struct point called `struct point ec_temp;`. And inside `point_mul` I call a function that turns the point `ec_temp` to zero. Therefore what I can do is anticipate that initialization and initialize `ec_temp` as "zero point" inside `ec_priv_to_pub`:
```rust
pub fn ec_priv_to_pub(k: [u8;20], Q: &Point, psp_ctx: &PSP_CTX) -> Result<(), Error> {
    let mut ec_temp = Point {
        x: BigUint::zero(),
        y: BigUint::zero(),
        is_infinity: true
    };
    //....
    // Rest of the code
}
```

And with that in mind I can just send it to `point_mul` already initialized as zero and then perform the `for-loop` from inside `point_mul`:

```cpp
static void point_mul(struct point *d, u8 *a, struct point *b)  // a is bignum
{
  //... More code
  for (i = 0; i < 21; i++)
    for (mask = 0x80; mask != 0; mask >>= 1) {
      point_double(d, d);
      if ((a[i] & mask) != 0)
        point_add(d, d, b);
    }
}
```

**Explanation about the for-loop:**
Because C does not have a native way to say "read the 15th bit of the 160-bit number", the dev had to build a machine to read the raw bytes one zero or one at a time. Here's how it works.:

**First, The Outer Loop:** `for (i = 0; i < 21; i++)`
The private key (a) is stores as an array of 21 bytes right? This loop grabs one byte at a time, starting from the first byte to the end of it (starting from the most significant, obviously).

**Second, The Inner Loop:** `for (mask = 0x80; masc != 0; mask >>= 1)`
This is the imporant part. **A byte has 8 bits**, and the hexadecimal number `0x80` written in binary is exactly `10000000`.
- The loop starts with the mask pointing at the far-left bit.
- `mask >>= 1` shifts that `1` one space to the right **every time the loop runs**
- So the mask turns into `01000000`, then after that `00100000`, all the way down to `00000001`. 

**Third, the condition:** `if ((a[i] & mask) != 0)`
The `&` (Bitwise AND) operator overlays the sliding mask onto the current byte of the private key. Because the mask only has a single 1 in it, the equation basically asks if the bit we are currently looking at is at 1 or 0.

**Fourth, the math: Double and Loop**
Inside this loop, the rules of Elliptic Curve multiplication are strictly followed:
- For each single bit scanned, the engine **always** doubles the current point (`point_double`).
- If the bit-scanner detects that the current bit is a `1` (not zero), it adds the base point to the total (`point_add`)
- If the bit is a `0`, it just skips the addition and moves to the next bit.


**Fifth, How the Engine Wakes up From Zero (The Infinity Trick)**
Since because `ec_temp` (or `d`) starts as the Point at Infinity (Geometric Zero), **the engine must respect two geometric rules at the beginning of the loop:**

1. Doubling Infinity just equals Infinity.
2. Adding a Point to Infinity equals to that Point.

> [!NOTE] Let's make an example
> Let's pretend that the first three bits of the Private Key are `101`, here is how exactly the engine build the multiplier from zero:
> - Start `d` is Infinity
> - Bit 1 is `1`:
> 	- **Always Double:** Double Infinity. `d` is still Infinity
> 	- **Add (Bit = 1):** Add the Base Point (`G`) to d. (`Infinity + G = 1G`)
> 	- **Result:** `d` is now `1G`
> - Bit 2 is `0`:
> 	- **Always double:** Double `d`. (`1G * 2 = 2G`)
> 	- **Skip Add (Bit = 0)**
> 	- **Result:** `d` is now `2G`
> - Bit 3 is `1`:
> 	- **Always Double:** Double `d`. (`2G * 2 = 4G`)
> 	- **Add (Bit = 1):** Add the Base Point (G) to d (`4G + 1G = 5G`)
> 	- **Result:** `d` is now 5G

##### Looking at documentation from BigUInt 
While searching through documentation, I stumble across interesting functions implemented inside `BigUInt` that could help me translate this perfectly without needing the `0x80` mask that scans each bit.

```rust
pub fn bit(&self, bit: [u64]) -> [bool](https://doc.rust-lang.org/nightly/std/primitive.bool.html)
Returns whether the bit in the given position is set
```

#### Before continuing
While I was performing a reverse engineering graph about how to translate the `point_double()` routine into `Rust`. **I stumble across this:**
```c
  elt_square(t, px);  // t = px*px
  elt_add(s, t, t); // s = 2*px*px
  elt_add(s, s, t); // s = 3*px*px
  elt_add(s, s, ec_a);  // s = 3*px*px + a
  elt_add(t, py, py); // t = 2*py
  elt_inv(t, t);    // t = 1/(2*py)
  elt_mul(s, s, t); // s = (3*px*px+a)/(2*py)

  elt_square(rx, s);  // rx = s*s
  elt_add(t, px, px); // t = 2*px
  elt_sub(rx, rx, t); // rx = s*s - 2*px

  elt_sub(t, px, rx); // t = -(rx-px)
  elt_mul(ry, s, t);  // ry = -s*(rx-px)
  elt_sub(ry, ry, py);  // ry = -s*(rx-px) - py
```
Which seems really weird to me, because at first It was understandable up until There. However I had to stop and see how exactly performs this. But at a first sight, It seems that is performing `BigUInt` operations, because `px` points to an `u8` slice of 20 length. And `20 bytes = 160 bit`, which is exactly the size of `BigUint` .


##### <u>Explanations</u>
<u>Explanation about</u> `elt_square(u8 *d, u8 *a)`:
- Calls `elt_mul(d, a, a)`

<u>Explanation about</u> `elt_add(u8 *d, u8 *a, u8 *b)`:
- Calls `bn_add(d, a, b, ec_p, 20);`
- Explanation about `bn_add()`
	- **Just as it says the function:** It performs `BigNumber` Addition, which I think I can replicate using `BigUint` vars by doing `a + b`

<u>Explanation about</u> `elt_inv(u8 *d, u8 *a)`:
- Creates a `u8` array of `20` bytes long called `s`.
- Calls `elt_copy(s, a);`
- Calls `bn_mon_inv(d, s, ec_p, 20);`

<u>Explanation about</u> `elt_mul(u8 *d, u8 *a, u8 *b)`:
- Calls `bn_mon_mul(d, a, b, ec_p, 20);`

**So basically, here:**
```c
  elt_square(t, px);  // t = px*px
  elt_add(s, t, t); // s = 2*px*px
  elt_add(s, s, t); // s = 3*px*px
```

Is trying to convert s into `3*px*px`. And we can translate this with this method:

```rust
let s = _mod_addition(t, t, p) // s =2*px*px
```

<u>But by using the mod_add function, let's remember this:</u>
In **Elliptic Curve Cryptography**, every single `X` and `Y` coordinate is trapped inside a finite field defined by the **prime number** `p`.
If the ceiling `p` is `100`, and I add `60+50`, the flat math will give me `110`. Right? but because I'm inside a finite field, it wraps around the ceiling and becomes `10` (`110 % 100 = 10`). And if it hits exactly `100` an addition operation, it becomes `0`.

**The C Memory Model**
While I was looking at the code from the function `point_double` I had to investigate more about 2011 C.
The original 2011 C code was constrained by memory. So returning large structs directly from functions in C causes the compiler **to copy the entire struct byte by byte, which is slow.** 
**In order to solve this, the C dev used the Output Pointer Pattern:**
- They passed a destination pointer (e.g. `struct point *r` or `u8 *d`) into the function.
- The only job for the function was to mutate that exact memory address in place.
- And this is why variables like `s` constantly overwrote themselves (e.g. mutating from $2x^2$ to $3x^2$ and so forth).

**The problem with Rust: Borrow Checker**
When translating the C signature `fn point_double (r: &mut Point, p: &Point)` to Rust, a major arquitectural problem resides inside the **Double and Loop** (`point_mul`).
- The engine needs **in-place doubling:** `point_double(d,d)`
- Rust's Borrow Checker strictly prohibits having a mutable reference (`&mut r`) and an immutable reference `&p` pointing to the exact same memory at the exact same time.
- So attempting to pass `d` as both parameters instantly triggers `Error` in order to prevent **memory corruption**.

**My final Architecture Design**
In order to satisfy Rust's memory safety while preserving the exact mathematical intent of making point_doubling to the same address of memory and also in order to prevent that other functions that I haven't discovered yet that could potentially use that specifically function for doing in-place doubling. I've decided to split that function into two more functions, which servers different ways of doing the same thing:

1. `fn point_double_out_place(r: &mut Point, p: &Point, ctx: &PSP_CTX)`
   - **Purpose:** Matches the same 1:1 C structure and now It's being used when the source point and destination point are different.
2. `fn point_double_in_place(p: &Point, ctx: &PSP_CTX)`
  - **Purpose:** It's built specifically for the `point_mul` loop where a point must double and overwrite itself without violating memory rules. 

---
#### Testing
```
~/Projects/tests_in_c/c_file
❯ ./c_test
Now we are setting the curve
x: 01D508CDC784DE50BDC70BE396E5EACBE1EE0685
y: E5EDED245D13500F31469F04ACF7211431BDCE6E⏎ 
```

Now knowing the results of point_double from the c file, I'm about to create a unit test file that compares the results from **Rust** to the **C** ones in order to see **if the translation was a success:**

**`struct point` that we are using:**

```cpp
struct point p = {
	.x = {
		0x22, 0x59, 0xAC, 0xEE, 0x15,
		0x48, 0x9C, 0xB0, 0x96, 0xA8,
		0x82, 0xF0, 0xAE, 0x1C, 0xF9,
		0xFD, 0x8E, 0xE5, 0xF8, 0xFA
	},
	.y = {
		0x60, 0x43, 0x58, 0x45, 0x6D,
		0x0A, 0x1C, 0xB2, 0x90, 0x8D,
		0xE9, 0x0F, 0x27, 0xD7, 0x5C,
		0x82, 0xBE, 0xC1, 0x08, 0xC0
	}
};
```


##### The test failed....
```
running 1 test
test test_point_double ... FAILED

failures:

---- test_point_double stdout ----
Rust x: [D6, 4E, E3, FF, C7, 11, D7, D1, 43, 4A, 9C, A0, 2E, 90, 09, 18, FE, 61, 66, 80]
Rust y: [20, 7A, 93, EA, 0B, 1B, F5, 94, 46, E2, 34, 5C, DA, 1F, 8E, 24, 79, A9, 3C, C5]

thread 'test_point_double' (35816) panicked at src/main.rs:131:5:
assertion `left == right` failed
  left: 185421280727456109961458342778573583399764180165
 right: 10459816419489798566181601858320480134293751429
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    test_point_double

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--bin ecdsa_experiment`

~/Projects/testing_rs/ecdsa_experiment master*
❯ 
```

The values from the experiment are different, therefore there must be something inside the functions that I'm calling inside point_double that are making this abnormality and doesn't let me get the same result as in C. Therefore I'm gonna create more unit tests but these ones are gonna be atomic and are gonna be the ones that point_double are calling:

```
point_double
├── mod_square
├── mod_add
├── mod_inv
├── mod_mul
└── mod_sub
```

So once all 5 passes and their results are the same with the corresponding C results. Then test point_double as a hole and compare.

#### `mod_square`
##### In C
Result that I grab from `test_square()`:
```
square: FA, 76, 20, 2E, 84, 78, 0A, E3, 11, 23, 77, 20, CD, 6B, 4D, 92, BB, 6F, 1F, 15,
```
**And the value that I'm using as experiment is:**
```
u8 x[20] = {
	0x22, 0x59, 0xAC, 0xEE, 0x15,
	0x48, 0x9C, 0xB0, 0x96, 0xA8,
	0x82, 0xF0, 0xAE, 0x1C, 0xF9,
	0xFD, 0x8E, 0xE5, 0xF8, 0xFA
};
```

##### In Rust
Used the same value and compare:
```
08 4B 13 95 70 72 98 51 C8 E2 80 93 41 29 8E 68 A6 1C F1 0F
```

They aren't the same... So the square multiplication failed or it isn't implemented correctly on the Rust's side.
H
C and Rust disagree. So let's find the first operation where their mathematical values diverge.

First: I tested bn_to_from & bn_from_mon:
```c
u8 x[20] {
	// a sequence of bytes
}
u8 original[20];
bn_to_mon(x, ec_p, 20);
bn_from_mon(x, ec_p, 20);
```

The result:
```c
Now we are setting the curve
original: 2259ACEE15489CB096A882F0AE1CF9FD8EE5F8FA
roundtrip: 2259ACEE15489CB096A882F0AE1CF9FD8EE5F8FA
```

So the Issue isn't inside the conversion from big number to Montgomery and vice-versa.

<u>So the Montgomery conversion works:</u>
$$
x \xrightarrow{\text{bn\_to\_mon }} xR \bmod p \xrightarrow{\text{bn\_from\_mon}} x
$$
>But doesn't prove that Montgomery multiplication is correct

Ok.... I think I know what was the issue (Im dumb). Look at EC_P on Rust:
```rust
pub static EC_P: [u8; 20] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 
    0xFF, 0xFF, 0x00, 0x00, 0x00, 
    0x01, 0xFF, 0xFF, 0xFF, 0xFF, 
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
```

**And from C (The original one):**
```c
static u8 ec_p[20] = {
	0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 
	0xFF, 0xFF, 0xFF, 0x00, 0x00, 
	0x00, 0x01, 0xFF, 0xFF, 0xFF, 
	0xFF, 0xFF, 0xFF, 0xFF, 0xFF
};
```

**Do you see the error?** In **Rust** we have seven `0xFF` bytes before the three zero bytes... And then 9 `0xFF` bytes after it... I think this was the issue that was giving me different results.

**Rust result:**
```rust
running 1 test
[40, 3A, 09, AD, 88, A2, 7B, 4B, 8F, 68, 56, 17, 5C, F7, 18, 19, BE, 18, A0, 9B]
```

**C Result:**
```c
❯ ./c_test
Now we are setting the curve
Square: 403A09AD88A27B4B8F6856175CF71819BE18A09B
```

THEY ARE THE SAME!!! BINGO





#### `mod_add`


#### `mod_inv`
#### `mod_mul`
#### `mod_sub`

I think the issue was that the set curve inside the test one and in rust also differ. Therefore I was using a set curve that wasn't the real one and I was getting different results because of that...

**The original one:**
```c
int kirk_CMD12(u8 * outbuff, int outsize) {
// ..code
  ecdsa_set_curve(ec_p,ec_a,ec_b2,ec_N2,Gx2,Gy2);
// ..more code
}
```

**The one that I was testing:**
```c
ecdsa_set_curve(ec_p,ec_a,ec_b1,ec_N1,Gx1,Gy1);
```
If you can see we have different set_curves...


**Ok so this is where I got stuck:**
```c
int kirk_CMD12(u8 * outbuff, int outsize) {
  u8 k[0x15];
  KIRK_CMD12_BUFFER * keypair = (KIRK_CMD12_BUFFER *) outbuff;

  if(outsize != 0x3C) return KIRK_INVALID_SIZE;
  ecdsa_set_curve(ec_p,ec_a,ec_b2,ec_N2,Gx2,Gy2);
  k[0] = 0;
  kirk_CMD14(k+1,0x14);
  ec_priv_to_pub(k, (u8*)keypair->public_key.x);
  memcpy(keypair->private_key,k+1,0x14);
  
  return KIRK_OPERATION_SUCCESS;
}
```

**So let's replicate it exactly** 
- Initialize `k` with size of `0x15`
- Do **Type Punning** for the `outbuff` pointer
- Set the respective curve with `ecdsa_set_curve()`
- Leave `kirk_cmd14()` behind, we replace it with `memset` (as a mock test).
- Print the result, copy it, make a unit test in Rust

**Result:**
```bash
Private key (k): 00FA4B7D3C47E8ED7E7EFA55ED26D2815B4D511B2E
```

**Now we have to replicate the same thing in Rust.**
I know that they did `0x15` rather than `0x14` because when you perform `montgomery aritmethic` and you are dealing with **big numbers**, it could lead to results that can go past the boundaries from a `0x14` pointer. Therefore they add another byte just in case while performing the arithmetic don't suffer a buffer overflow.

##### Important
I spotted an interesting comment while moving a static variable from c to Rust:
```c
static u8 ec_b2[20] = {0xA6, 0x8B, 0xED, 0xC3, 0x34, 0x18, 0x02, 0x9C, 0x1D, 0x3C, 0xE3, 0x3B, 0x9A, 0x32, 0x1F, 0xCC, 0xBB, 0x9E, 0x0F, 0x0B};// mon
```
> Look at the comment: `// mon`.

This could mean that the value from `ec_b2` is already set into the Montgomery Form. Since in Rust I'm performing another arithmetic operation rather than using the Montgomery form, I'm gonna convert these values back into `BigNumber` using an existing routine inside the original tool called `bn_from_mon()`.

**So the `u8` arrays that suffer the same issue are the listed below:**
- `ec_a` (encountered the same comment)
- `ec_b2` (explained above)

> I think those are the only arrays that enters in the same bag









