I just take as an inspiration the implementation that I did from `point_double_out_place` but without using `p` as the data that is being use for calculation of the slope. But while I was performing this implementation, I had to rewrite it multiple times since It was highly unnefficient And I was copying entire `BigUInt` structs when there was another way of implement things. Here was the journey of implementing this function:

When translating the C function to Rust, I encountered a big difference between in-place and out-place. In C we pass a pointer to the result r and then read/write to r->x and r->y throughout the function.

#### Hypothesis 1: Cloning (The Slow Approach but a safer one)
The initial thought was that, in order to avoid data corruption and satisfy the borrow checker from rust, we must clone the original coordinates into variables that are independent from `r`:
```rust
let original_x = r.x.clone()
let original_y = r.y.clone()
// ... And then do math by using original_x and original_y
r.x = new_x;
r.y = new_y;
```
**Advantages:** It's 100% safe and the compiler wouldn't argue with us. 
**Disadvantages:** Cloning `BigUInt` requieres heap allocation. And by doing that inside a small function like `point_mul` could introduce performance penalties, since we are literally calling this routine hundreds of times while performing `point_mul`.

#### Hypothesis 2: Use intermediate Immutable references
The second thought was, Since BigUInt is only a wrapper around memory, we don't necessarily need to copy the data. We can just create immutable references pointing to the original data that resides inside r. Like this:
```rust
let px = &.x;
let py = &.x;
// ... do math using px and py ...
r.x = new_x;
r.y = new_y;
```
**Advantages:** ZERO ALLOCATIONS. We only pass pointers!!!
**Disadvantages:** **Unnecessary variables.** Creating `px` and `py` just to hold references that points to `r.x` and `r.y` is not that important, since we aren't modifying `r.x` before calculating the slope, the modification comes after it.

#### Final Design: Use read operations to `r` instead
Thanks to Rust, the compiler is smart to know that we are only reading `r.x` and `r.y` when we are performing the calculation of the slope. So this is my final decision design for this function.